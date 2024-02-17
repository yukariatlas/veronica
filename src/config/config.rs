use std::option::Option;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub db_path: String,
    pub portfolio_path: String,
    pub finmind_token: String,
}

impl std::default::Default for Config {
    fn default() -> Self {
        Config {
            db_path: "".to_owned(),
            portfolio_path: "".to_owned(),
            finmind_token: "".to_owned(),
        }
    }
}

pub fn load_config(config_path: &str) -> Option<Config> {
    let data = std::fs::read_to_string(config_path).ok();

    if data.is_none() {
        return None;
    }
    serde_yaml::from_str(&data.unwrap()).ok()
}

