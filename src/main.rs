use std::{collections::HashMap, error::Error, fmt::Display, str::FromStr};

use bytes::Bytes;
use clap::Parser;
use http::response::Builder;
use http_body_util::Full;
use hyper::{body, server::conn::http1, service::service_fn, Request, Response};

mod client;
mod client_pool;
use client::{Client, GeoLocation, LocationInfo, ScrapeError, SearchResult};
use client_pool::{new_client_pool, ObjectPool, PoolError};
use hyper_util::rt::{TokioIo, TokioTimer};
use serde::Serialize;
use serde_json::json;
use tokio::net::TcpListener;

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

#[derive(Debug)]
enum HandlerError {
    ScrapeError(ScrapeError),
    PoolError(PoolError),
    QueryError(String),
}

impl Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandlerError::ScrapeError(e) => write!(f, "ScrapeError({})", e),
            HandlerError::PoolError(e) => write!(f, "PoolError({})", e),
            HandlerError::QueryError(e) => write!(f, "QueryError({})", e),
        }
    }
}

impl From<ScrapeError> for HandlerError {
    fn from(value: ScrapeError) -> Self {
        HandlerError::ScrapeError(value)
    }
}

impl From<PoolError> for HandlerError {
    fn from(value: PoolError) -> Self {
        HandlerError::PoolError(value)
    }
}

impl From<url::ParseError> for HandlerError {
    fn from(value: url::ParseError) -> Self {
        HandlerError::QueryError(format!("failed to parse URL: {}", value))
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
                    let result = handle_search(pool, req).await;
                    Ok(api_result_to_response(Response::builder(), result)?)
                } else if req.uri().path() == "/api/reviews" {
                    Ok(Response::new("REVIEWS RESULTS HERE".into()))
                } else {
                    Response::builder().status(404).body("404 not found".into())
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

async fn handle_search(
    pool: ObjectPool<Client>,
    request: Request<body::Incoming>,
) -> Result<Vec<LocationInfo>, HandlerError> {
    let args = Query::parse(&request)?;

    let client = pool.get().await?;
    let location = GeoLocation {
        latitude: args.get("latitude")?,
        longitude: args.get("longitude")?,
        accuracy: args.get("accuracy")?,
    };
    Ok(
        match client
            .search(&args.get::<String>("query")?, &location)
            .await?
        {
            SearchResult::NotFound => vec![],
            SearchResult::Singular(x) => vec![x],
            SearchResult::Multiple(x) => x,
        },
    )
}

struct Query {
    map: HashMap<String, String>,
}

impl Query {
    fn parse(request: &Request<body::Incoming>) -> Result<Self, HandlerError> {
        let query = request
            .uri()
            .query()
            .ok_or_else(|| HandlerError::QueryError("missing query string".to_owned()))?;
        let mut value = HashMap::new();
        for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
            value.insert(k.into(), v.into());
        }
        Ok(Self { map: value })
    }

    fn get<T: FromStr>(&self, k: &str) -> Result<T, HandlerError>
    where
        T::Err: Display,
    {
        if let Some(val) = self.map.get(k) {
            T::from_str(val).map_err(|x| {
                HandlerError::QueryError(format!("failed to parse argument {}: {}", k, x))
            })
        } else {
            Err(HandlerError::QueryError(format!("no argument: {}", k)))
        }
    }
}

fn api_result_to_response<T: Serialize, E: Error + Display>(
    builder: Builder,
    result: Result<T, E>,
) -> Result<Response<Full<Bytes>>, http::Error> {
    match result {
        Ok(x) => match serde_json::to_string(&x) {
            Ok(x) => builder.body(x.into()),
            Err(x) => builder.status(500).body(
                serde_json::to_string(&json!({"error": format!("failed to encode result: {}", x)}))
                    .unwrap()
                    .into(),
            ),
        },
        Err(x) => builder.body(
            serde_json::to_string(&json!({"error": format!("{}", x)}))
                .unwrap()
                .into(),
        ),
    }
}
