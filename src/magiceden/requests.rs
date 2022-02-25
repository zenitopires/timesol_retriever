use tokio::time::Duration;

#[tokio::main]
pub async fn get_collection_names() -> Option<serde_json::Value> {
    let endpoint = "https://api-mainnet.magiceden.dev/all_collections";

    let mut res = surf::get(&endpoint).await.ok()?;

    if res.status() == surf::StatusCode::TooManyRequests {
        println!("Too many request sent. Sleeping for 1 minute.");
        tokio::time::sleep(Duration::from_secs(60)).await;
        res = surf::get(&endpoint).await.ok()?;
        dbg!(res.status());
    }

    let res_content = res.body_json().await.ok()?;

    res_content
}

