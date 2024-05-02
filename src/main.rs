use std::{convert::Infallible, error::Error};

use bytes::Bytes;
use clap::Parser;
use futures::{pin_mut, select, FutureExt};
use http::response::Builder;
use http_body_util::{combinators::BoxBody, Full};
use hyper::{body, server::conn::http1, service::service_fn, Request, Response};

mod client;
mod client_pool;
mod handlers;
use client::Client;
use client_pool::{new_client_pool, ObjectPool};
use handlers::{api_result_to_response, handle_reviews, handle_search};
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::{net::TcpListener, signal};

const PAGE_MAPPING: [(&'static str, &'static str); 13] = [
    ("", include_str!("assets/index.html")),
    ("/", include_str!("assets/index.html")),
    ("/404.html", include_str!("assets/404.html")),
    ("/js/app.js", include_str!("assets/js/app.js")),
    ("/js/app.js.map", include_str!("assets/js/app.js.map")),
    ("/ts/app.ts", include_str!("assets/ts/app.ts")),
    ("/js/search.js", include_str!("assets/js/search.js")),
    ("/js/search.js.map", include_str!("assets/js/search.js.map")),
    ("/ts/search.ts", include_str!("assets/ts/search.ts")),
    ("/css/page.css", include_str!("assets/css/page.css")),
    ("/css/search.css", include_str!("assets/css/search.css")),
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    let pool = new_client_pool(1, &args.driver).await?;
    let result = entrypoint(args, &pool).await;

    pool.close(|client| client.close()).await?;

    result
}

async fn entrypoint(
    args: Args,
    pool: &ObjectPool<Client>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
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
        let io = TokioIo::new(tcp);

        let local_pool = pool.clone();

        let make_service = service_fn(move |req: Request<body::Incoming>| {
            let pool = local_pool.clone();
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
                } else {
                    for (page, content) in PAGE_MAPPING {
                        if req.uri().path() == page {
                            return Ok(static_response(Response::builder(), content)?);
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
