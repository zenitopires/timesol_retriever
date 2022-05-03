use log::{debug, info};

pub const ME_MAX_REQUESTS: usize = 120;

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
