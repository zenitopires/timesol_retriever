use std::error::Error;

use crate::utils;
use utils::config_reader::Config;

pub struct Database {
    pub client: tokio_postgres::Client
}
//
// impl DB {

impl Database {
    pub async fn connect(db_config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let cnxn_str = format!("host={} user={} password={} dbname={}",
                               db_config.host, db_config.user, db_config.password, db_config.dbname);

        let (client, connection) = tokio_postgres::connect(cnxn_str.as_str(), tokio_postgres::NoTls
        ).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Ok(Self {
            client
        })
    }
}

// pub fn execute_query(client: DB, query: &str) -> Result<Vec<postgres::Row>, postgres::Error> {
// }