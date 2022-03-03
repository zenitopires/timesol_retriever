#![allow(unreachable_code)]

use std::error::Error;
use std::time::Duration;

use log::{info, trace, debug, warn};
use env_logger;
use tokio_postgres;
use surf::StatusCode;
use tokio_postgres::types::Timestamp;
use futures::stream::{self, StreamExt};
use built;

mod utils;
use utils::config_reader::{Config, read_file, parse_yaml};

mod magiceden;
use magiceden::requests::{get_collection_names};
use magiceden::parse::{parse_collection_names, parse_collection_stats};

mod database;
use database::db_connection::Database;
use crate::stream::FuturesUnordered;

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[cfg(feature = "chrono")]
fn built_time() -> built::chrono::DateTime<built::chrono::Local> {
    built::util::strptime(built_info::BUILT_TIME_UTC)
        .with_timezone(&built::chrono::offset::Local)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Starting {}, version: {}", built_info::PKG_NAME, built_info::PKG_VERSION);
    info!("Host: {}", built_info::HOST);
    info!("Package authors: {}", built_info::PKG_AUTHORS);
    info!("Repository: {}", built_info::PKG_REPOSITORY);
    #[cfg(feature = "semver")]
    debug!("{:?}", built_info::DEPENDENCIES);
    info!("Beginning retrieval...");

    let config = match read_file("C:\\Users\\Zenito\\CLionProjects\\solgraph_backend\\src\\secrets.yaml") {
        Ok(contents) => contents,
        Err(e) => panic!("Could not read file. Reason: {:?}", e)
    };

    let db_config: Config = parse_yaml(config);

    dbg!(&db_config);

    let database = Database::connect(db_config).await?;

    let data  = tokio::task::spawn_blocking(|| {
        match get_collection_names() {
            Some(value) => value,
            None => serde_json::from_str("{}").unwrap()
        }
    }).await.expect("Thread panicked!");

    let collection_names = parse_collection_names(data);

    // Enable TimescaleDB Extension
    database.client.execute("CREATE EXTENSION IF NOT EXISTS timescaledb", &[]).await?;

    database.client.execute(
    "
    CREATE TABLE IF NOT EXISTS collection_names (
        symbol varchar(255) NOT NULL,
        PRIMARY KEY (symbol)
    )
    ", &[]).await?;

    for symbol in &collection_names {
        database.client.query("INSERT INTO collection_names(symbol) VALUES ($1) ON CONFLICT DO NOTHING", &[&symbol]).await?;
    }

    let rows = database.client.query("SELECT symbol FROM collection_names", &[]).await?;

    let mut present_collections: Vec<String> = Vec::new();

    for row in rows {
        let name: &str = row.get(0);
        present_collections.push(name.to_string().clone());
    }

    database.client.execute(
        "
            CREATE TABLE IF NOT EXISTS collection_stats (
                symbol varchar(255) NOT NULL,
                floor_price double precision,
                total_volume double precision,
                total_listed bigint,
                avg_24h_price double precision,
                date TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )
            ", &[]).await?;
    database.client.execute("SELECT create_hypertable('collection_stats', 'date', chunk_time_interval => INTERVAL '1 Day', if_not_exists => TRUE)", &[]).await?;
    let collection_stats = database.client.query("SELECT symbol FROM collection_stats", &[]).await?;

    let mut stat_collections: Vec<String> = Vec::new();
    for row in collection_stats {
        stat_collections.push(row.get(0));
    }

    let mut urls = vec![];

    for name in &collection_names {
        let url = format!("https://api-mainnet.magiceden.dev/v2/collections/{}/stats", name);
        urls.push(url.clone());
    }

    let mut futs = FuturesUnordered::new();
    loop {
        // TODO: Update collection names
        for url in &urls {
            futs.push(surf::get(url));

            // Once we reach 100 requests, await current batch
            if futs.len() == 118 {
                while let Some(res) = futs.next().await {
                    let data: serde_json::Value = match res.unwrap().body_json().await {
                        Ok(value) => value,
                        Err(_e) => serde_json::from_str("{}").unwrap()
                    };

                    let magiceden_stats = parse_collection_stats(data);

                    debug!("{:?}", magiceden_stats);

                    database.client.execute("
                        INSERT INTO collection_stats
                        (
                            symbol,
                            floor_price,
                            total_volume,
                            total_listed,
                            avg_24h_price,
                            date
                        )
                    VALUES
                        ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
                    ON CONFLICT DO NOTHING
                    ",
                                            &[
                                                &magiceden_stats.symbol,
                                                &magiceden_stats.floor_price,
                                                &magiceden_stats.volume_all,
                                                &magiceden_stats.listed_count,
                                                &magiceden_stats.avg_price]).await?; //{
                    //     Ok(value) => {
                    //         println!("Success: {}", value) },
                    //     Err(e) => {
                    //         panic!("Failed to insert data into collection_stats: {}", e)
                    //     }
                    // }
                }
                info!("Waiting a minute to avoid TooManyRequests HTTP error");
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        }
    }

    Ok(())
}