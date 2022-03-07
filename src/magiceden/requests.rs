use log::{info, debug};

pub const ME_MAX_REQUESTS: usize = 100;

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

// #[tokio::main]
// pub async fn get_collection_stats(collection_name: String) -> Result<(serde_json::Value, surf::StatusCode), Box<dyn Error>> {
//     let endpoint = format!("https://api-mainnet.magiceden.dev/v2/collections/{}/stats", collection_name);

    // let magiceden_res = surf::get(&endpoint).await;

    // let mut res: surf::Response= match magiceden_res {
    //     Ok(value) => value,
    //     Err(e) => { panic!("Error: {}", e) }
    // };

    // let stats: serde_json::Value  = match res.body_json().await.ok() {
    //     Some(val) => val,
    //     None => serde_json::from_str("{}").unwrap()
    // };
    // // dbg!(&stats, res.status());

    // // if res.status() == surf::StatusCode::TooManyRequests {
    // //     println!("Too many request sent. Sleeping for 1 minute.");
    // //     tokio::time::sleep(Duration::from_secs(60)).await;
    // //     res = surf::get(&endpoint).await.ok()?;
    // //     dbg!(res.status());
    // // }
    // //
    // // let res_content = res.body_json().await.ok()?;

//     Ok((stats, res.status()))
// }
