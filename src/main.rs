mod domain;
mod server;
mod utils;

use domain::Config;
use log::{error, info};

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    info!("start dmz_locker begin");

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

    if let Err(e) = config.validate() {
        error!("Error starting server: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = server::start(config).await {
        error!("Error starting server: {}", e);
        std::process::exit(1);
    }
}
