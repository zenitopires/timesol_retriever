mod api;
mod database;

use api::api_connection::{self, Connection};
use database::db_connection::DB;

use serde::{Deserialize, Serialize};

// use postgres;

use tokio_postgres;

use std::error::Error;
use std::time::Duration;
use surf::StatusCode;

mod aggregation;

use aggregation::{Summary, Tweet, NewsPaper};

#[derive(Deserialize, Serialize)]
struct Device {
    id: u32,
    manufacturer: String
}

const LAMPORTS_PER_SOL: f64 = 1000000000.0;

// #[tokio::main]
// async fn main() -> Result<(), surf::Error> {
//
//
//     let client = surf::Client::new();
//
//     let client_new = Connection::new();
//
//     let res1: serde_json::Value = client_new.client.recv_json(
//             surf::get("https://api-mainnet.magiceden.dev/v2/collections?offset=0&limit=500")).await?;
//
//     dbg!(res1);
//
//
//     // let mut res1: serde_json::Value = client.recv_json(
//     //     surf::get("https://random-data-api.com/api/device/random_device")).await?;
//
//     // dbg!(res1);
//
//     // let mut res = surf::get("https://random-data-api.com/api/device/random_device").await?;
//
//     // let Device { manufacturer , id} = res.body_json().await?;
//
//     // println!("id: {} manufacturer: {}", id, manufacturer);
//
//     // let manu = data["manufacturer"].is_string();
//
//     // if manu {
//     //     println!("It's a string!");
//     //     match data["manufacturer"].as_str() {
//     //         Some(value) => {
//     //             println!("Manufacturer: {}", value);
//     //         },
//     //         None => {
//     //             println!("No value found for manufacturer!");
//     //         }
//     //     }
//     // }
//
//     // println!();
//
//     Ok(())
// }

#[derive(Debug)]
struct Collection {
    floorPrice: f64,
    listedCount: i64,
    symbol: String,
    volumeAll: f64
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    let (client, connection) = tokio_postgres::connect("host=localhost user=postgres \
    password=admin dbname=magiceden", tokio_postgres::NoTls
    ).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let data  = tokio::task::spawn_blocking(|| {
        match get_collection_names() {
            Some(value) => value,
            None => serde_json::from_str("{}").unwrap()
        }
    }).await.expect("Thread panicked!");

    let mut collection_names: Vec<String> = Vec::new();

    match data.as_object() {
        Some(obj) => {
            match obj["collections"].as_array() {
                Some(collections) => {
                    for collection in collections {
                        let symbol = collection["symbol"].as_str().unwrap();
                        collection_names.push(symbol.to_string());
                    }
                },
                None => { println!("nothing"); }
            }
        },
        None => { println!("Nothing"); }
    }

    client.execute(
    "
    CREATE TABLE IF NOT EXISTS collection_names (
        symbol varchar(255) NOT NULL,
        PRIMARY KEY (symbol)
    )
    ", &[]).await?;

    for symbol in collection_names {
        client.query("INSERT INTO collection_names(symbol) VALUES ($1) ON CONFLICT DO NOTHING", &[&symbol]).await?;
    }

    let value: String;
    let rows = client.query("SELECT symbol FROM collection_names", &[]).await?;

    let mut present_collections: Vec<String> = Vec::new();

    for row in rows {
        let mut name: &str = row.get(0);
        present_collections.push(name.to_string().clone());
    }

    client.execute(
        "
            CREATE TABLE IF NOT EXISTS collection_stats (
                symbol varchar(255) NOT NULL,
                floor_price double precision,
                total_volume double precision,
                total_listed bigint,
                avg_24h_price double precision,
                PRIMARY KEY (symbol)
            )
            ", &[]).await?;
    let collection_stats = client.query("SELECT symbol FROM collection_stats", &[]).await?;

    let mut stat_collections: Vec<String> = Vec::new();
    for row in collection_stats {
        stat_collections.push(row.get(0));
    }

    let mut count = 0;
    let collection_count = present_collections.len();
    for name in present_collections {
        if count < collection_count {
            if stat_collections.contains(&name) {
                count += 1;
                continue
            }

            let endpoint = format!("https://api-mainnet.magiceden.dev/v2/collections/{}/stats", name);

            let mut res = surf::get(&endpoint).await?;
            dbg!(res.status());

            if res.status() == StatusCode::TooManyRequests {
                println!("Too many request sent. Sleeping for 1 minute.");
                tokio::time::sleep(Duration::from_secs(60)).await;
                res = surf::get(&endpoint).await?;
                dbg!(res.status());
            }

            let stats: serde_json::Value = res.body_json().await?;

            dbg!(&stats);

            let symbol = match stats.get("symbol") {
                Some(value) => match stats["symbol"].as_str() {
                    Some(value) => value,
                    None => "unknown symbol"
                },
                None => "unknown symbol"
            };

            let avg_price = match stats.get("avgPrice24hr") {
                Some(value) => match stats["avgPrice24hr"].as_f64() {
                    Some(value) => value / LAMPORTS_PER_SOL,
                    None => 0.0
                },
                None => 0.0
            };

            let floor_price = match stats.get("floorPrice") {
                Some(value) =>  match stats["floorPrice"].as_f64() {
                    Some(value) => value / LAMPORTS_PER_SOL,
                    None => 0.0
                },
                None => 0.0
            };

            let listed_count = match stats.get("listedCount") {
                Some(value) => match stats["listedCount"].as_i64() {
                    Some(value) => value,
                    None => 0i64
                },
                None => 0i64
            };

            let volume_all = match stats.get("volumeAll") {
                Some(value) => match stats["volumeAll"].as_f64() {
                    Some(value) => value / LAMPORTS_PER_SOL,
                    None => 0.0
                },
                None => 0.0
            };

            client.execute(
                "
                INSERT INTO collection_stats
                    (
                    symbol,
                    floor_price,
                    total_volume,
                    total_listed,
                    avg_24h_price
                    )
                VALUES
                    ($1, $2, $3, $4, $5)
                ON CONFLICT DO NOTHING
                ",
                &[&symbol, &floor_price, &volume_all, &listed_count, &avg_price]).await?;
        }
        count += 1;
        // dbg!(res_content);
    }

    // for d in data {
    //     dbg!(d);
    // }

    Ok(())
}

#[tokio::main]
async fn get_collection_names() -> Option<serde_json::Value> {
    let mut res = surf::get("https://api-mainnet.magiceden.dev/all_collections").await.ok()?;

    let res_content = res.body_json().await.ok()?;

    res_content
}

#[tokio::main]
async fn get_collection_stats(collection_name: String) -> Option<serde_json::Value> {
    let endpoint = format!("https://api-mainnet.magiceden.dev/v2/collections/{}/stats", collection_name);

    let mut res = surf::get(endpoint).await.ok()?;

    let res_content = res.body_json().await.ok()?;

    res_content
}
