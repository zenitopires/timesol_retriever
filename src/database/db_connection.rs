use std::error::Error;

use futures::stream::FuturesOrdered;
use futures::StreamExt;
use surf::RequestBuilder;
use tokio::time::Duration;
use tracing::{debug, error, info, warn};

use crate::utils;
use utils::config_reader::Config;

use crate::magiceden::parse::parse_collection_stats;
use crate::magiceden::requests::ME_MAX_REQUESTS;

pub struct Database {
    pub client: tokio_postgres::Client,
}

impl Database {
    pub async fn connect(db_config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let cnxn_str = format!(
            "host={} user={} password={} dbname={}",
            db_config.host, db_config.user, db_config.password, db_config.dbname
        );

        let (client, connection) =
            tokio_postgres::connect(cnxn_str.as_str(), tokio_postgres::NoTls).await?;

        tokio::spawn(async move {
            match connection.await {
                Ok(success) => {
                    println!("{:?}", success);
                    info!("DB connection was successful");
                }
                Err(e) => {
                    error!("Connection failed! Reason: {}", e);
                }
            }
        });

        Ok(Self { client })
    }

    pub async fn initialize_database(&self) -> Result<(), Box<dyn Error>> {
        //        println!("Going ham");
        // Enable TimescaleDB Extension
        //       self.client
        //           .execute("CREATE EXTENSION IF NOT EXISTS timescaledb", &[])
        //           .await?;

        self.client
            .execute(
                "
        CREATE TABLE IF NOT EXISTS collection_names (
            symbol      varchar(255)    NOT NULL,
            PRIMARY KEY (symbol)
        )
    ",
                &[],
            )
            .await?;

        self.client
            .execute(
                "
        CREATE TABLE IF NOT EXISTS retriever_state (
            symbol          varchar(255),
            finished_loop   boolean,
            symbol_id       bigint,
            time            TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY     (symbol_id)
        )",
                &[],
            )
            .await?;

        // Insert some data so that we can update something later on
        self.client
            .execute(
                "
        INSERT INTO retriever_state (symbol, finished_loop, symbol_id, time)
        VALUES('empty', false, '1', CURRENT_TIMESTAMP)
        ON CONFLICT DO NOTHING
        ",
                &[],
            )
            .await?;

        self.client
            .execute(
                "
        CREATE TABLE IF NOT EXISTS collection_stats (
            symbol          varchar(255)        NOT NULL,
            floor_price     double precision,
            total_volume    double precision,
            total_listed    bigint,
            avg_24h_price   double precision,
            date            TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        )",
                &[],
            )
            .await?;

        //       self.client
        //           .execute(
        //               "
        //       SELECT create_hypertable(
        //           'collection_stats',
        //           'date',
        //           chunk_time_interval => INTERVAL '1 Day',
        //           if_not_exists => TRUE
        //       )",
        //               &[],
        //           )
        //           .await?;

        Ok(())
    }

    pub async fn last_known_collection(&self) -> Option<String> {
        let mut last_known_collection: &str = "";
        let empty: i64 = match self
            .client
            .query("SELECT COUNT(*) FROM retriever_state", &[])
            .await
        {
            Ok(row) => row[0].get(0),
            Err(e) => {
                panic!("Could not fetch last retriever state. Reason {}", e);
            }
        };
        let row = match self
            .client
            .query("SELECT symbol, finished_loop FROM retriever_state", &[])
            .await
        {
            Ok(row) => row,
            Err(e) => {
                panic!("Could not fetch last known collection. Reason {}", e);
            }
        };
        if empty != 0 {
            let symbol_temp: &str = row[0].get(0);
            last_known_collection = symbol_temp.clone();
        }
        Some(last_known_collection.to_string())
    }

    pub async fn reset_rt_state(&self) -> Result<(), Box<dyn Error>> {
        self.client
            .execute(
                "
                DELETE from retriever_state
                WHERE symbol_id = 1
                ",
                &[],
            )
            .await?;
        self.client
            .execute(
                "
                INSERT INTO retriever_state (symbol, finished_loop, symbol_id, time)
                VALUES('empty', false, '1', CURRENT_TIMESTAMP)
                ON CONFLICT DO NOTHING
                ",
                &[],
            )
            .await?;
        Ok(())
    }

    pub async fn check_futures(
        &self,
        futs: &mut FuturesOrdered<RequestBuilder>,
    ) -> Result<(), Box<dyn Error>> {
        if futs.len() == ME_MAX_REQUESTS {
            info!(
                "Reached max number of collections received. \
                Attempting to unload data into database..."
            );
            self.insert_batch(futs).await?;
            // insert_batch(, futs, unknown_symbols, &mut data_inserted).await?;

            info!("Waiting a minute to avoid TooManyRequests HTTP error");
            tokio::time::sleep(Duration::from_secs(60)).await;
        }

        Ok(())
    }

    async fn insert_batch(
        &self,
        futs: &mut FuturesOrdered<RequestBuilder>,
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
                            continue;
                        } else {
                            self.client
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
}
