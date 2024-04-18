use std::error::Error;

use clap::Parser;

mod client;
mod client_pool;
use client::{Client, GeoLocation, SearchResult};

#[derive(Parser, Clone)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long, value_parser, default_value = "http://localhost:9515")]
    driver: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    let client = Client::new(&args.driver).await?;
    let location = GeoLocation {
        latitude: 37.63,
        longitude: -122.44,
        accuracy: 10.0,
    };
    let results = client.search("Grand Hyatt Tampa Bay", &location).await?;
    println!("got results: {:?}", results);
    match results {
        SearchResult::Singular(x) => {
            println!("getting reviews for {}", x.name);
            println!("{:?}", client.list_reviews(&x.url, &location).await?);
        }
        _ => {}
    }
    client.close().await?;
    Ok(())
}
