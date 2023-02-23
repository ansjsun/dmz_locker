use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicU32, AtomicU64},
        Arc,
    },
};

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Deserialize, Clone)]
pub struct Config {
    pub port: u16,
    pub authentication: String,
    pub mappings: Vec<Mapping>,
    pub session_time_sec: u64,
}

#[derive(Default, Debug, Deserialize, Clone)]
pub struct Mapping {
    pub port: u16,
    pub addr: String,
    pub is_public: bool,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct ClientInfo {
    pub remote_ip: String,
    pub port: u16,
    pub count: Arc<AtomicU32>,
    pub last_time: Arc<AtomicU64>,
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.port == 0 {
            return Err("port is required".to_string());
        }
        if self.authentication.is_empty() {
            return Err("authentication is required".to_string());
        }
        if self.mappings.is_empty() {
            return Err("mappings is required".to_string());
        }

        self.mappings.iter().try_for_each(|mapping| {
            if mapping.port == 0 {
                return Err("port is required".to_string());
            }
            if mapping.addr.is_empty() {
                return Err("addr is required".to_string());
            }
            Ok(())
        })?;

        //check mapping port is unique
        let mut mapping_ports = self
            .mappings
            .iter()
            .map(|mapping| mapping.port)
            .collect::<Vec<u16>>();

        let unique = mapping_ports.iter().cloned().collect::<HashSet<u16>>();

        if unique.len() != mapping_ports.len() {
            mapping_ports.sort();
            return Err(format!(
                "mapping port is not unique ports:{:?}",
                mapping_ports
            ));
        }

        Ok(())
    }
}
