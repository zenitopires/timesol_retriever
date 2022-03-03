use std::error::Error;
use log::{info, warn, error};

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

        let (client, connection) = match tokio_postgres::connect(cnxn_str.as_str(),
                                                             tokio_postgres::NoTls
        ).await {
            Ok(value) => value,
            Err(e) => { error!("Failed to connect to {}", db_config.dbname); panic!("Failed to \
            connect. Reason: {}", e);}
        };

        // match connection.await {
        //     Ok(success) => { info!("DB connection was successful"); },
        //     Err(e) => { warn!("Connection failed!"); }
        // }

        tokio::spawn(async move {
            println!("HELLO");
            match connection.await {
                Ok(success) => { println!("{:?}", success); info!("DB connection was successful")
                ; },
                Err(e) => { warn!("Connection failed!"); }
            }
            // if let Err(e) = connection.await {
            //     eprintln!("connection error: {}", e);
            // } else {
            //     info!("DB connection successful.");
            // }
        });

        Ok(Self {
            client
        })
    }
}

// pub fn execute_query(client: DB, query: &str) -> Result<Vec<postgres::Row>, postgres::Error> {
// }