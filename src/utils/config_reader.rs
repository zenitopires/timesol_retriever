use std::fs;
use std::io::{self, Read};

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub host: String,
    pub user: String,
    pub password: String,
    pub dbname: String,
}

pub fn read_file(file_path: &str) -> Result<String, io::Error> {
    let f = fs::File::open(file_path);
    let mut f = match f {
        Ok(file) => file,
        Err(err) => panic!("Failed to open config file! Reason: {:?}", err),
    };

    let mut contents: String = String::new();

    match f.read_to_string(&mut contents) {
        Ok(_) => Ok(contents),
        Err(err) => panic!("Failed to read file into string: {:?}", err),
    }
}
