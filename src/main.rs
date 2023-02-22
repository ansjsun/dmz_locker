mod domain;
mod server;
mod utils;

use std::{env::Args, net::TcpListener, sync::Arc, thread::JoinHandle};

use domain::Config;
use log::error;
use serde::Deserialize;

fn main() {
    env_logger::init();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or(String::from("conf/config.toml"));

    let data = match std::fs::read(config_path.as_str()) {
        Ok(data) => data,
        Err(e) => {
            error!("Error reading config file: {}", e);
            std::process::exit(1);
        }
    };

    let config_str = match String::from_utf8(data) {
        Ok(c) => c,
        Err(e) => {
            error!("Error reading config file: {} has err:{:?}", config_path, e);
            std::process::exit(1);
        }
    };

    let config = toml::from_str::<Config>(&config_str).unwrap();

    if let Err(e) = server::start(config) {
        error!("Error starting server: {}", e);
        std::process::exit(1);
    }
}
