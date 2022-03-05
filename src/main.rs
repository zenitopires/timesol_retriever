#![allow(unreachable_code)]

use std::error::Error;
use std::time::Duration;

// use log::{info, debug, error};
// use env_logger;
use futures::stream::{self, StreamExt};

use tracing::{info, debug, error, warn, Level};
use tracing_subscriber;

use tracing_appender;

mod utils;
use utils::config_reader::{Config, read_file, parse_yaml};

mod magiceden;
use magiceden::requests::{get_collection_names};
use magiceden::parse::{parse_collection_names, parse_collection_stats};

mod database;
use database::db_connection::Database;
use crate::magiceden::requests::ME_MAX_REQUESTS;
use crate::stream::FuturesUnordered;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // let file_appender = tracing_appender::rolling::hourly("C:\\Users\\Zenito\\Desktop", "retriever.log");
    // let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    // tracing_subscriber::fmt().with_max_level(Level::TRACE).with_ansi(false).with_writer(non_blocking).init();
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    info!("Starting {}, version: {}", built_info::PKG_NAME, built_info::PKG_VERSION);
    info!("Host: {}", built_info::HOST);
    info!("Built for {}", built_info::TARGET);
    info!("Package authors: {}", built_info::PKG_AUTHORS);
    info!("Repository: {}", built_info::PKG_REPOSITORY);
    info!("Beginning collection retrieval...");

    let config = match read_file("C:\\Users\\Zenito\\CLionProjects\\solgraph_backend\\src\\secrets\
    .yaml") {
        Ok(contents) => contents,
        Err(e) => panic!("Could not read file. Reason: {:?}", e)
    };

    let db_config: Config = parse_yaml(config);

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

    let mut urls = vec![];

    for name in &collection_names {
        let url = format!("https://api-mainnet.magiceden.dev/v2/collections/{}/stats", name);
        urls.push(url.clone());
    }

    let mut unknown_symbols = 0;
    let mut futs = FuturesUnordered::new();
    loop {
        // TODO: Update collection names
        for url in &urls {
            futs.push(surf::get(url));

            // Once we reach 100 requests, await current batch
            let mut data_inserted = 0;
            if futs.len() == ME_MAX_REQUESTS {
                info!("Reached max number of collections received. \
                Attempting to unload data into database...");
                while let Some(res) = futs.next().await {
                    let data: serde_json::Value = match res.unwrap().body_json().await {
                        Ok(value) => value,
                        Err(e) => {
                            warn!("Error occured: {}. Will use an empty JSON.", e);
                            serde_json::from_str("{}").unwrap()
                        }
                    };

                    let magiceden_stats = parse_collection_stats(data);

                    debug!("{:?}", magiceden_stats);

                    if magiceden_stats.symbol == "unknown symbol" {
                        unknown_symbols += 1;
                    } else {
                        database.client.execute("
                        INSERT INTO collection_stats
                        (symbol, floor_price, total_volume,
                         total_listed, avg_24h_price, date)
                        VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
                            ON CONFLICT DO NOTHING
                    ", &[
                            &magiceden_stats.symbol,
                            &magiceden_stats.floor_price,
                            &magiceden_stats.volume_all,
                            &magiceden_stats.listed_count,
                            &magiceden_stats.avg_price]).await?;
                        data_inserted += 1;
                    }
                }
                info!("Unknown collections received: {}. {}", unknown_symbols,
                    if unknown_symbols > 0 { "Will not add them to database." }
                    else { "" });
                info!("Number of collections inserted into database: {}/{}", data_inserted, ME_MAX_REQUESTS);
                info!("Waiting a minute to avoid TooManyRequests HTTP error");
                unknown_symbols = 0;
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        }
    }

    Ok(())
}