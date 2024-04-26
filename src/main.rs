use std::{error::Error, fmt::Display};

use bytes::Bytes;
use clap::Parser;
use http_body_util::Full;
use hyper::{body, server::conn::http1, service::service_fn, Request, Response};

mod client;
mod client_pool;
use client::{Client, GeoLocation, ScrapeError, SearchResult};
use client_pool::{new_client_pool, ObjectPool};
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::net::TcpListener;

#[derive(Parser, Clone)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long, value_parser, default_value = "http://localhost:9515")]
    driver: String,

    #[clap(long, value_parser, default_value = ":8080")]
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

#[derive(Debug)]
enum HandlerError {
    NotFound,
    ScrapeError(ScrapeError),
}

impl Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandlerError::NotFound => write!(f, "page not found"),
            HandlerError::ScrapeError(e) => write!(f, "ScrapeError({})", e),
        }
    }
}

impl Error for HandlerError {}

async fn entrypoint(
    args: Args,
    pool: &ObjectPool<Client>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind(&args.host).await?;
    loop {
        let (tcp, _) = listener.accept().await?;
        let io = TokioIo::new(tcp);

        let local_pool = pool.clone();

        let make_service = service_fn(move |req: Request<body::Incoming>| {
            let pool = local_pool.clone();
            async move {
                if req.uri().path() == "/api/search" {
                    Ok(Response::new(Full::<Bytes>::from("SEARCH RESULTS HERE")))
                } else if req.uri().path() == "/api/reviews" {
                    Ok(Response::new(Full::<Bytes>::from("REVIEWS RESULTS HERE")))
                } else {
                    Err(HandlerError::NotFound)
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

    // let mut client = pool.get().await;
    // let location = GeoLocation {
    //     latitude: 37.63,
    //     longitude: -122.44,
    //     accuracy: 10.0,
    // };
    // let results = client.search("Death by Taco", &location).await?;
    // println!("got results: {:?}", results);
    // match results {
    //     SearchResult::Singular(x) => {
    //         println!("getting reviews for {}", x.name);
    //         let mut count = 0;
    //         let mut review_it = client.list_reviews(&x.url, &location).await?;
    //         while let Some(result) = review_it.next().await? {
    //             println!("{:?}", result);
    //             count += result.len();
    //             println!("seen {} results so far", count);
    //         }
    //     }
    //     _ => {}
    // }
    // Ok(())
}
