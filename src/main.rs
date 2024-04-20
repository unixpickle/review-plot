use std::error::Error;

use clap::Parser;

mod client;
mod client_pool;
use client::{GeoLocation, SearchResult};
use client_pool::new_client_pool;

#[derive(Parser, Clone)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long, value_parser, default_value = "http://localhost:9515")]
    driver: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    let pool = new_client_pool(1, &args.driver).await?;
    let client = pool.get().await;
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
