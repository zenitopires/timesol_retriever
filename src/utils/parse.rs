use serde_yaml;

use crate::utils::config_reader::Config;

pub fn parse_yaml(yaml_file: String) -> Config {
    let deserialized_config: Config = match serde_yaml::from_str(&yaml_file) {
        Ok(value) => value,
        Err(e) => panic!("Failed to deserialize yaml: {:?}", e),
    };
    deserialized_config
}