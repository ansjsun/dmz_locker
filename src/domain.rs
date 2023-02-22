use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Deserialize, Clone)]
pub struct Config {
    pub port: u16,
    pub authentication: String,
    pub mappings: Vec<Mapping>,
}

#[derive(Default, Debug, Deserialize, Clone)]
pub struct Mapping {
    pub port: u16,
    pub addr: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct ClientInfo {
    pub remote_addr: String,
    pub port: u16,
    pub count: u32,
    pub last_time: u64,
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
            return Err("mapping port is not unique".to_string());
        }

        Ok(())
    }
}