use std::fmt::{Display, Formatter};

const LAMPORTS_PER_SOL: f64 = 1000000000.0;

#[derive(Debug)]
pub struct MagicEdenCollection {
    pub symbol: String,
    pub avg_price: f64,
    pub floor_price: f64,
    pub listed_count: i64,
    pub volume_all: f64,
}

impl Display for MagicEdenCollection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "symbol: {}\navg_price: {}\nfloor_price: {}\nlisted_count: {}\nvolume_all: {}\n",
        self.symbol, self.avg_price, self.floor_price, self.listed_count, self.volume_all)
    }
}

pub fn parse_collection_names(data: serde_json::Value) -> Vec<String> {
    let mut collection_names: Vec<String> = Vec::new();

    match data.as_object() {
        Some(obj) => match obj["collections"].as_array() {
            Some(collections) => {
                for collection in collections {
                    let symbol = collection["symbol"].as_str().unwrap();
                    collection_names.push(symbol.to_string());
                }
            }
            None => {
                println!("nothing");
            }
        },
        None => {
            println!("Nothing");
        }
    }

    collection_names
}

pub fn parse_collection_stats(stats: serde_json::Value) -> MagicEdenCollection {
    // let endpoint = format!("https://api-mainnet.magiceden.dev/v2/collections/{}/stats", name);
    //
    // let mut res = surf::get(&endpoint).await?;
    // dbg!(res.status());
    //
    // if res.status() == surf::StatusCode::TooManyRequests {
    //     println!("Too many request sent. Sleeping for 1 minute.");
    //     tokio::time::sleep(Duration::from_secs(60)).await;
    //     res = surf::get(&endpoint).await?;
    //     dbg!(res.status());
    // }
    //
    // let stats: serde_json::Value = res.body_json().await?;

    // dbg!(&stats);

    let symbol = match stats.get("symbol") {
        Some(_value) => match stats["symbol"].as_str() {
            Some(value) => value,
            None => "unknown symbol",
        },
        None => "unknown symbol",
    };

    let avg_price = match stats.get("avgPrice24hr") {
        Some(_value) => match stats["avgPrice24hr"].as_f64() {
            Some(value) => value / LAMPORTS_PER_SOL,
            None => 0.0,
        },
        None => 0.0,
    };

    let floor_price = match stats.get("floorPrice") {
        Some(_value) => match stats["floorPrice"].as_f64() {
            Some(value) => value / LAMPORTS_PER_SOL,
            None => 0.0,
        },
        None => 0.0,
    };

    let listed_count = match stats.get("listedCount") {
        Some(_value) => match stats["listedCount"].as_i64() {
            Some(value) => value,
            None => 0i64,
        },
        None => 0i64,
    };

    let volume_all = match stats.get("volumeAll") {
        Some(_value) => match stats["volumeAll"].as_f64() {
            Some(value) => value / LAMPORTS_PER_SOL,
            None => 0.0,
        },
        None => 0.0,
    };

    MagicEdenCollection {
        symbol: String::from(symbol),
        avg_price,
        floor_price,
        listed_count,
        volume_all,
    }
}
