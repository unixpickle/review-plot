use std::error::Error;

use clap::Parser;

mod client;
use client::{Client, GeoLocation};

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
    let results = client
        .search(
            "Grand Hyatt",
            &GeoLocation {
                latitude: 37.63,
                longitude: -122.44,
                accuracy: 10.0,
            },
        )
        .await?;
    println!("got results: {:?}", results);
    Ok(())
}
