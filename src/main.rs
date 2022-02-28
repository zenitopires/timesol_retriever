use tokio_postgres;
use std::error::Error;
use std::time::Duration;
use surf::StatusCode;
use tokio_postgres::types::Timestamp;

mod utils;
use utils::config_reader::{Config, read_file, parse_yaml};

mod magiceden;
use magiceden::requests::{get_collection_names};
use magiceden::parse::{parse_collection_names, parse_collection_stats};

mod database;
use database::db_connection::Database;
use crate::magiceden::requests::get_collection_stats;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = match read_file("/Users/zenito/solgraph_backend/src/secrets.yaml") {
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

    // database.client.execute("SELECT 'CREATE DATABASE magiceden' WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = \'magiceden\')\\gexec", &[]).await?;

    // Enable TimescaleDB Extension
    database.client.execute("CREATE EXTENSION IF NOT EXISTS timescaledb", &[]).await?;

    database.client.execute(
    "
    CREATE TABLE IF NOT EXISTS collection_names (
        symbol varchar(255) NOT NULL,
        PRIMARY KEY (symbol)
    )
    ", &[]).await?;

    for symbol in collection_names {
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

    // tokio::task::spawn_blocking(|| {
    //     dbg!(get_collection_stats(String::from("pawnshop_gnomies")).unwrap());
    // });

    let mut requests: Vec<_> = Vec::new();

    // tokio::task::spawn_blocking(|| {
    //     dbg!(get_collection_stats(String::from("pawnshop_gnomies")).unwrap());
    // });

    let mut count = 0;
    for name in present_collections {
        if count < 100 {
            let stats = tokio::task::spawn_blocking(|| {
                match get_collection_stats(String::from(name)) {
                    Ok(value) => {
                        let (data, res) = value;
                        (data, res)
                    },
                    Err(_) => serde_json::from_str("{}").unwrap()
                }
            });

            requests.push(stats);
            count += 1;
        }
    }

    for task in requests {
        match task.await {
            Ok(value) => {
                let (data, res) = value;
                if res == surf::StatusCode::TooManyRequests {
                    println!("Will sleep for a minute");
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
                dbg!(data);
            },
            Err(e) => { panic!("Failure: {}", e); }
        }
    }


    // let mut count = 1;
    // for task in requests {
    //     dbg!(count);
        // let magiceden_stats = parse_collection_stats(task.await.unwrap());
        // if magiceden_stats.symbol == "unknown symbol" {
        //     continue
        // }
        // dbg!(task.await);
        // database.client.execute("
        //     INSERT INTO collection_stats
        //     (
        //         symbol,
        //         floor_price,
        //         total_volume,
        //         total_listed,
        //         avg_24h_price,
        //         date
        //     )
        // VALUES
        //     ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
        // ON CONFLICT DO NOTHING
        // ",
        //                         &[
        //                             &magiceden_stats.symbol,
        //                             &magiceden_stats.floor_price,
        //                             &magiceden_stats.volume_all,
        //                             &magiceden_stats.listed_count,
        //                             &magiceden_stats.avg_price]).await?;
        // count += 1;
    // }

    // let mut count = 0;
    // let collection_count = present_collections.len();
    // loop {
    //     for name in &present_collections {
    //         if count < collection_count {
    //
    //             dbg!(format!("Getting data about: {}", name));
    //             let endpoint = format!("https://api-mainnet.magiceden.dev/v2/collections/{}/stats", name);
    //
    //             let mut res = surf::get(&endpoint).await?;
    //
    //             if res.status() == StatusCode::NotFound {
    //                 continue;
    //             }
    //
    //             if res.status() == StatusCode::TooManyRequests {
    //                 println!("Too many request sent. Sleeping for 1 minute.");
    //                 tokio::time::sleep(Duration::from_secs(60)).await;
    //                 res = surf::get(&endpoint).await?;
    //                 dbg!(res.status());
    //             }
    //
    //             let stats: serde_json::Value = res.body_json().await?;
    //
    //             let magiceden_stats = parse_collection_stats(stats);
    //
    //             dbg!(&magiceden_stats);
    //
    //             database.client.execute(
    //                 "
    //             INSERT INTO collection_stats
    //                 (
    //                 symbol,
    //                 floor_price,
    //                 total_volume,
    //                 total_listed,
    //                 avg_24h_price,
    //                 date
    //                 )
    //             VALUES
    //                 ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
    //             ON CONFLICT DO NOTHING
    //             ",
    //                 &[
    //                     &magiceden_stats.symbol,
    //                     &magiceden_stats.floor_price,
    //                     &magiceden_stats.volume_all,
    //                     &magiceden_stats.listed_count,
    //                     &magiceden_stats.avg_price]).await?;
    //         }
    //         count += 1;
    //     }
    //     // TODO: Get new collections here?
    //     count = 0;
    // }

    Ok(())
}