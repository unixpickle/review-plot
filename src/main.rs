use std::error::Error;

use clap::Parser;

mod client;
mod client_pool;
use client::{Client, GeoLocation, SearchResult};
use client_pool::{new_client_pool, ObjectPool};

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
    let result = entrypoint(&pool).await;

    for client in pool.drain().await {
        client.close().await?;
    }

    result
}

async fn entrypoint(pool: &ObjectPool<Client>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut client = pool.get().await;
    let location = GeoLocation {
        latitude: 37.63,
        longitude: -122.44,
        accuracy: 10.0,
    };
    let results = client.search("Death by Taco", &location).await?;
    println!("got results: {:?}", results);
    match results {
        SearchResult::Singular(x) => {
            println!("getting reviews for {}", x.name);
            let mut count = 0;
            let mut review_it = client.list_reviews(&x.url, &location).await?;
            while let Some(result) = review_it.next().await? {
                println!("{:?}", result);
                count += result.len();
                println!("seen {} results so far", count);
            }
        }
        _ => {}
    }
    Ok(())
}
