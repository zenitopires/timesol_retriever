use std::error::Error;
use tracing::{error, info};

use crate::utils;
use utils::config_reader::Config;

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
        println!("Going ham");
        // Enable TimescaleDB Extension
        self.client
            .execute("CREATE EXTENSION IF NOT EXISTS timescaledb", &[])
            .await?;

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

        self.client
            .execute(
                "
        SELECT create_hypertable(
            'collection_stats',
            'date',
            chunk_time_interval => INTERVAL '1 Day',
            if_not_exists => TRUE
        )",
                &[],
            )
            .await?;

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
        self
            .client
            .execute(
                "
                DELETE from retriever_state
                WHERE symbol_id = 1
                ",
                &[],
            )
            .await?;
        self
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
        Ok(())
    }
}
