use surf;
use serde_json;

pub struct Connection {
    pub client: surf::Client,
    // data: serde_json::Value
}

impl Connection {
    pub fn new() -> Self {
        Self { client: surf::Client::new() }
    }
}