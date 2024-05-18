use std::{convert::Infallible, error::Error, sync::Arc};

use bytes::Bytes;
use clap::Parser;
use futures::{pin_mut, select, FutureExt};
use http::response::Builder;
use http_body_util::{combinators::BoxBody, Full};
use hyper::{body, server::conn::http1, service::service_fn, Request, Response};

mod client;
mod client_pool;
mod geolocate;
mod handlers;
use client::Client;
use client_pool::{new_client_pool, ObjectPool};
use handlers::{api_result_to_response, handle_reviews, handle_search};
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::{net::TcpListener, signal};

use crate::geolocate::IpLocator;

const PAGE_MAPPING: [(&'static str, &'static str); 21] = [
    ("", include_str!("assets/index.html")),
    ("/", include_str!("assets/index.html")),
    ("/404.html", include_str!("assets/404.html")),
    ("/js/app.js", include_str!("assets/js/app.js")),
    ("/js/app.js.map", include_str!("assets/js/app.js.map")),
    ("/ts/app.ts", include_str!("assets/ts/app.ts")),
    ("/js/search.js", include_str!("assets/js/search.js")),
    ("/js/search.js.map", include_str!("assets/js/search.js.map")),
    ("/ts/search.ts", include_str!("assets/ts/search.ts")),
    ("/js/location.js", include_str!("assets/js/location.js")),
    (
        "/js/location.js.map",
        include_str!("assets/js/location.js.map"),
    ),
    ("/ts/location.ts", include_str!("assets/ts/location.ts")),
    ("/js/plot.js", include_str!("assets/js/plot.js")),
    ("/js/plot.js.map", include_str!("assets/js/plot.js.map")),
    ("/ts/plot.ts", include_str!("assets/ts/plot.ts")),
    ("/css/page.css", include_str!("assets/css/page.css")),
    ("/css/location.css", include_str!("assets/css/location.css")),
    ("/css/search.css", include_str!("assets/css/search.css")),
    ("/css/plot.css", include_str!("assets/css/plot.css")),
    ("/css/loader.css", include_str!("assets/css/loader.css")),
    ("/css/404.css", include_str!("assets/css/404.css")),
];

#[derive(Parser, Clone)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long, value_parser, default_value = "http://localhost:9515")]
    driver: String,

    #[clap(long, value_parser, default_value = "0.0.0.0:8080")]
    host: String,

    #[clap(long, value_parser, default_value_t = 0)]
    num_proxies: usize,

    #[clap(long, short, action)]
    headless: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    let pool = new_client_pool(1, &args.driver, args.headless).await?;
    let result = entrypoint(args, &pool).await;

    pool.close(|client| client.close()).await?;

    result
}

async fn entrypoint(
    args: Args,
    pool: &ObjectPool<Client>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let locator = Arc::new(IpLocator::new(args.num_proxies));
    let listener = TcpListener::bind(&args.host).await?;
    let exit_signal = signal::ctrl_c().fuse();
    pin_mut!(exit_signal);
    loop {
        let tcp;
        let accept = listener.accept().fuse();
        pin_mut!(accept);
        select! {
            x = accept => tcp = x?.0,
            _ = exit_signal => {
                println!("Got interrupt; stopping server.");
                return Ok(());
            }
        }
        let client_ip = format!("{}", tcp.peer_addr().expect("get peer address").ip());
        let io = TokioIo::new(tcp);

        let local_pool = pool.clone();
        let local_locator = locator.clone();

        let make_service = service_fn(move |req: Request<body::Incoming>| {
            let pool = local_pool.clone();
            let local_locator = local_locator.clone();
            let local_client_ip = client_ip.clone();
            async move {
                if req.uri().path() == "/api/search" {
                    let result = handle_search(pool, req).await;
                    api_result_to_response(Response::builder(), result)
                } else if req.uri().path() == "/api/reviews" {
                    match handle_reviews(pool, req).await {
                        Err(e) => {
                            api_result_to_response(Response::builder(), Result::<String, _>::Err(e))
                        }
                        Ok(x) => Ok(x),
                    }
                } else if req.uri().path() == "/api/location" {
                    let location = local_locator.lookup_for_request(&req, &local_client_ip);
                    api_result_to_response(
                        Response::builder(),
                        Result::<Option<(f64, f64)>, Infallible>::Ok(location),
                    )
                } else {
                    for (page, content) in PAGE_MAPPING {
                        if req.uri().path() == page {
                            let content_type = match page.split(".").last().unwrap() {
                                "css" => "text/css",
                                "/" | "html" => "text/html",
                                "js" => "application/javascript",
                                _ => "text/plain",
                            };
                            return Ok(static_response(
                                Response::builder().header("content-type", content_type),
                                content,
                            )?);
                        }
                    }
                    Ok(static_response(
                        Response::builder().status(404),
                        include_str!("assets/404.html"),
                    )?)
                }
            }
        });

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .timer(TokioTimer::new())
                .serve_connection(io, make_service)
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}

fn static_response(
    builder: Builder,
    data: &str,
) -> Result<Response<BoxBody<Bytes, Infallible>>, http::Error> {
    builder.body(BoxBody::new(Full::<Bytes>::from(data.to_owned())))
}
