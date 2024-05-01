use std::{collections::HashMap, convert::Infallible, error::Error, fmt::Display, str::FromStr};

use bytes::Bytes;
use futures::StreamExt;
use http::response::Builder;
use http_body_util::{combinators::BoxBody, Full, StreamBody};
use hyper::{
    body::{self, Frame},
    Request, Response,
};

use super::client::{Client, GeoLocation, LocationInfo, ScrapeError, SearchResult};
use super::client_pool::{ObjectPool, PoolError};
use serde::Serialize;
use serde_json::json;
use tokio::sync::mpsc::channel;
use tokio_stream::wrappers::ReceiverStream;

#[derive(Debug)]
pub enum HandlerError {
    ScrapeError(ScrapeError),
    PoolError(PoolError),
    HttpError(http::Error),
    QueryError(String),
}

impl Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandlerError::ScrapeError(e) => write!(f, "ScrapeError({})", e),
            HandlerError::PoolError(e) => write!(f, "PoolError({})", e),
            HandlerError::HttpError(e) => write!(f, "HttpError({})", e),
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

impl From<http::Error> for HandlerError {
    fn from(value: http::Error) -> Self {
        HandlerError::HttpError(value)
    }
}

impl From<url::ParseError> for HandlerError {
    fn from(value: url::ParseError) -> Self {
        HandlerError::QueryError(format!("failed to parse URL: {}", value))
    }
}

impl Error for HandlerError {}

pub async fn handle_search(
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

pub async fn handle_reviews(
    pool: ObjectPool<Client>,
    request: Request<body::Incoming>,
) -> Result<Response<BoxBody<Bytes, Infallible>>, HandlerError> {
    let args = Query::parse(&request)?;

    let location = GeoLocation {
        latitude: args.get("latitude")?,
        longitude: args.get("longitude")?,
        accuracy: args.get("accuracy")?,
    };
    let url = args.get::<String>("url")?;
    let mut client = pool.get().await?;

    let (tx, rx) = channel::<Bytes>(1);

    tokio::spawn(async move {
        match client.list_reviews(&url, &location).await {
            Err(e) => {
                tx.send(Bytes::from(
                    serde_json::to_string(&json!({"error": format!("{}", e)})).unwrap() + "\n",
                ))
                .await
                .ok();
            }
            Ok(mut it) => loop {
                match it.next().await {
                    Err(e) => {
                        tx.send(Bytes::from(
                            serde_json::to_string(&json!({"error": format!("{}", e)})).unwrap()
                                + "\n",
                        ))
                        .await
                        .ok();
                        return;
                    }
                    Ok(Some(x)) => {
                        if !tx
                            .send(Bytes::from(serde_json::to_string(&x).unwrap() + "\n"))
                            .await
                            .is_ok()
                        {
                            return;
                        }
                    }
                    Ok(None) => return,
                }
            },
        }
    });

    Ok(Response::new(BoxBody::new(StreamBody::new(
        ReceiverStream::from(rx).map(|x| -> Result<_, Infallible> { Ok(Frame::data(x)) }),
    ))))
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

pub fn api_result_to_response<T: Serialize, E: Error + Display>(
    builder: Builder,
    result: Result<T, E>,
) -> Result<Response<BoxBody<Bytes, Infallible>>, http::Error> {
    match result {
        Ok(x) => match serde_json::to_string(&x) {
            Ok(x) => builder.body(BoxBody::new(Full::<Bytes>::from(x))),
            Err(x) => builder.status(500).body(BoxBody::new(Full::<Bytes>::from(
                serde_json::to_string(&json!({"error": format!("failed to encode result: {}", x)}))
                    .unwrap(),
            ))),
        },
        Err(x) => builder.body(BoxBody::new(Full::<Bytes>::from(
            serde_json::to_string(&json!({"error": format!("{}", x)})).unwrap(),
        ))),
    }
}
