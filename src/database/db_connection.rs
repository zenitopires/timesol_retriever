use log::{info, warn, error};

use crate::utils;
use utils::config_reader::Config;

pub struct Database {
    pub client: tokio_postgres::Client
}

impl Database {
    pub async fn connect(db_config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let cnxn_str = format!("host={} user={} password={} dbname={}",
                               db_config.host, db_config.user, db_config.password, db_config.dbname);

        let (client, connection) = match tokio_postgres::connect(cnxn_str.as_str(),
                                                             tokio_postgres::NoTls
        ).await {
            Ok(value) => { info!("Connection to DB was successful."); value },
            Err(e) => { error!("Failed to connect to {}", db_config.dbname); panic!("Failed to \
            connect. Reason: {}", e);}
        };

        tokio::spawn(async move {
            match connection.await {
                Ok(success) => { println!("{:?}", success); info!("DB connection was successful")
                ; },
                Err(e) => { warn!("Connection failed! Reason: {}", e); }
            }
        });

        Ok(Self {
            client
        })
    }
}