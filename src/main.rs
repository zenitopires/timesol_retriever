#![allow(unreachable_code)]

use std::error::Error;
use std::time::Duration;

use futures::stream::{FuturesOrdered, StreamExt};

use tracing::{debug, info, trace, warn, Level};
use tracing_subscriber;

use tracing_appender;
use tracing_subscriber::registry::Data;

mod utils;
use utils::config_reader::{read_file, Config};
use utils::parse::parse_yaml;
use utils::pkg_info::pkg_info;

mod magiceden;
use magiceden::parse::{parse_collection_names, parse_collection_stats};
use magiceden::requests::get_collection_names;

mod database;

use magiceden::requests::ME_MAX_REQUESTS;
use database::db_connection::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let log_path = match std::env::var("trace_path") {
        Ok(val) => val,
        Err(e) => {
            panic!("Issue with trace_path! Reason: {}", e);
        }
    };
    let file_appender = tracing_appender::rolling::hourly(log_path, "retriever.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .with_ansi(false)
        .with_writer(non_blocking)
        .init();
    // tracing_subscriber::fmt().with_max_level(Level::TRACE).init();
    pkg_info();

    let config_path = match std::env::var("config_path") {
        Ok(val) => val,
        Err(e) => {
            panic!("Issue with config_path! Reason: {}", e);
        }
    };

    let config = match read_file(config_path.as_str()) {
        Ok(contents) => contents,
        Err(e) => panic!("Could not read file. Reason: {:?}", e),
    };

    let db_config: Config = parse_yaml(config);

    let database = Database::connect(db_config).await?;

    database.initialize_database().await?;

    let collections = tokio::task::spawn_blocking(|| match get_collection_names() {
        Some(value) => value,
        None => serde_json::from_str("{}").unwrap(),
    })
    .await
    .expect("Thread panicked!");

    let collection_names = parse_collection_names(collections);

    for symbol in &collection_names {
        database
            .client
            .query(
                "
            INSERT INTO collection_names(symbol)
            VALUES ($1) ON CONFLICT DO NOTHING",
                &[&symbol],
            )
            .await?;
    }

    let mut urls = vec![];

    for name in &collection_names {
        let url = format!(
            "https://api-mainnet.magiceden.dev/v2/collections/{}/stats",
            name
        );
        urls.push(url.clone());
    }

    let mut finished_loop: bool = false;
    let mut unknown_symbols = 0;
    let mut last_known_collection = match database.last_known_collection().await {
        Some(value) => value,
        None => String::from(""),
    };
    let mut futs = FuturesOrdered::new();
    loop {
        if finished_loop {
            finished_loop = false;
            database
                .client
                .execute(
                    "
                DELETE from retriever_state
                WHERE symbol_id = 1
                ",
                    &[],
                )
                .await?;
            database
                .client
                .execute(
                    "
                INSERT INTO retriever_state (symbol, finished_loop, symbol_id, time)
                VALUES('empty', false, '1', CURRENT_TIMESTAMP)
                ON CONFLICT DO NOTHING
                ",
                    &[],
                )
                .await?;
        }
        // TODO: Update collection names
        for url in &urls {
            if last_known_collection != "" && last_known_collection != "empty" {
                if url.as_str() != last_known_collection {
                    trace!("url does not match last known url before crash! Skipping");
                    continue;
                } else {
                    last_known_collection = String::from("");
                    info!("Continuing from last known collection: {}", url);
                }
            }

            futs.push(surf::get(url));

            database
                .client
                .execute(
                    "
                UPDATE retriever_state
                    SET symbol = $1, finished_loop = $2, time = CURRENT_TIMESTAMP
                WHERE symbol_id = 1
                ",
                    &[url, &finished_loop],
                )
                .await?;

            let mut data_inserted = 0;
            if futs.len() == ME_MAX_REQUESTS {
                info!(
                    "Reached max number of collections received. \
                Attempting to unload data into database..."
                );
                while let Some(res) = futs.next().await {
                    match res {
                        Ok(ref val) => {
                            if val.status() == surf::StatusCode::Ok {
                                info!("Status OK. Attempting to get stats from MagicEden.");
                                let data: serde_json::Value = match res.unwrap().body_json().await {
                                    Ok(value) => value,
                                    Err(e) => {
                                        warn!("Error occured: {}. Will use an empty JSON.", e);
                                        serde_json::from_str("{}").unwrap()
                                    }
                                };

                                let magiceden_stats = parse_collection_stats(data);

                                debug!("{:?}", &magiceden_stats);

                                if magiceden_stats.symbol == "unknown symbol" {
                                    unknown_symbols += 1;
                                } else {
                                    database
                                        .client
                                        .execute(
                                            "
                        INSERT INTO collection_stats
                        (symbol, floor_price, total_volume,
                         total_listed, avg_24h_price, date)
                        VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
                            ON CONFLICT DO NOTHING
                    ",
                                            &[
                                                &magiceden_stats.symbol,
                                                &magiceden_stats.floor_price,
                                                &magiceden_stats.volume_all,
                                                &magiceden_stats.listed_count,
                                                &magiceden_stats.avg_price,
                                            ],
                                        )
                                        .await?;
                                    data_inserted += 1;
                                }
                            } else {
                                warn!("Status was not OK. Skipping.");
                                continue;
                            }
                        }
                        Err(e) => {
                            warn!("Issue with response: {}", e);
                        }
                    }
                }
                info!(
                    "Unknown collections received: {}. {}",
                    unknown_symbols,
                    if unknown_symbols > 0 {
                        "Will not add them to database."
                    } else {
                        ""
                    }
                );
                info!(
                    "Number of collections inserted into database: {}/{}",
                    data_inserted, ME_MAX_REQUESTS
                );
                info!("Waiting a minute to avoid TooManyRequests HTTP error");
                unknown_symbols = 0;
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        }
        info!("Iteration of collection complete. Reiteration beginning soon.");
        finished_loop = true;
    }

    Ok(())
}
