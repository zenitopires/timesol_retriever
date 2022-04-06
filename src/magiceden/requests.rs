use log::{debug, info, trace, warn};
use std::error::Error;
use surf::RequestBuilder;

pub const ME_MAX_REQUESTS: usize = 100;

use crate::{parse_collection_stats, Database, FuturesOrdered, StreamExt};
use tokio::time::Duration;

#[tokio::main]
pub async fn get_collection_names() -> Option<serde_json::Value> {
    let endpoint = "https://api-mainnet.magiceden.dev/all_collections";
    info!("Getting collection names from {}", &endpoint);

    let mut res = surf::get(&endpoint).await.ok()?;
    debug!("{:?}", res);

    if res.status() == surf::StatusCode::TooManyRequests {
        info!("Waiting a minute to avoid TooManyRequests HTTP error");
        tokio::time::sleep(Duration::from_secs(60)).await;
        res = surf::get(&endpoint).await.ok()?;
        if res.status() == surf::StatusCode::Ok {
            info!("Received request successfully");
            debug!("{:?}", res);
        }
    }

    let res_content = res.body_json().await.ok()?;

    res_content
}

pub async fn check_futures(
    database: &Database,
    futs: &mut FuturesOrdered<RequestBuilder>,
    unknown_symbols: &mut i32,
) -> Result<(), Box<dyn Error>> {
    let mut data_inserted = 0;
    if futs.len() == ME_MAX_REQUESTS {
        info!(
            "Reached max number of collections received. \
                Attempting to unload data into database..."
        );
        insert_batch(&database, futs, unknown_symbols, &mut data_inserted).await?;
        info!(
            "Unknown collections received: {}. {}",
            unknown_symbols,
            if *unknown_symbols > 0 {
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
        *unknown_symbols = 0;
        tokio::time::sleep(Duration::from_secs(60)).await;
    }

    Ok(())
}

pub async fn insert_batch(
    database: &Database,
    futs: &mut FuturesOrdered<RequestBuilder>,
    data_inserted: &mut i32,
    unknown_symbols: &mut i32,
) -> Result<(), Box<dyn Error>> {
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
                        *unknown_symbols += 1;
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
                        *data_inserted += 1;
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

    Ok(())
}
