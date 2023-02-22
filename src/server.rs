use std::{
    collections::HashMap,
    io::Read,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, RwLock},
    thread::{spawn, JoinHandle},
    time::Duration,
    vec,
};

use log::error;
use moka::sync::Cache;

use crate::{
    domain::{ClientInfo, Config},
    utils::{self, current_seconds},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug)]
struct Server {
    servers: RwLock<Vec<JoinHandle<()>>>,
    conns: RwLock<HashMap<String, u64>>,
    white_list: RwLock<Cache<String, u64>>,
    black_list: RwLock<Cache<String, RwLock<ClientInfo>>>,
    errors: Vec<String>,
}

pub fn start(conf: Config) -> Result<()> {
    let server = Arc::new(Server {
        servers: RwLock::new(Default::default()),
        conns: RwLock::new(Default::default()),
        white_list: RwLock::new(
            Cache::builder()
                .time_to_idle(Duration::from_secs(60 * 60 * 120))
                .max_capacity(1000)
                .build(),
        ),
        black_list: RwLock::new(
            Cache::builder()
                .time_to_idle(Duration::from_secs(60 * 60 * 20))
                .max_capacity(1000)
                .build(),
        ),
        errors: vec![],
    });

    if server.errors.len() > 0 {
        return Err(server.errors.join("\n").into());
    }

    let listener = TcpListener::bind(format!("127.0.0.1:{}", conf.port)).unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = server.handle_inner(stream) {
                    error!("handle interal hs error: {}", e);
                }
            }
            Err(e) => return Err(format!("listener on autho server recive error: {}", e).into()),
        }
    }
    Ok(())
}

impl Server {
    fn start_listener(self: &Arc<Self>, port: u16, target_addr: Option<String>) {
        let server = self.clone();
        self.servers.write().unwrap().push(spawn(move || {}));
    }
}

impl Server {
    fn handle_inner(
        self: &Arc<Self>,
        authentication: &String,
        mut stream: TcpStream,
    ) -> Result<()> {
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;

        let remote_addr = stream.peer_addr()?.ip().to_string();

        let mut buf = vec![0_u8; 1024];

        let len = stream.read(&mut buf)?;
        buf.resize(len, 0);

        let content = String::from_utf8(buf)?;

        let head = utils::http_parse(&content);

        if let Some(path) = head.get("path") {
            if authentication.eq(path) {
            } else {
                return Err(format!(
                    "remote_addr:{:?} auth error , request path:{:?}",
                    remote_addr, path
                )
                .into());
            }
        }

        if head.get("path").unwrap() == "/proxy" {
            let addr = head.get("addr").unwrap();
            self.handle_proxy(stream, addr);
        } else {
            self.handle_inner(stream);
        }

        Ok(())
    }

    pub fn handle_proxy(self: &Arc<Self>, stream: TcpStream, addr: &String) {
        todo!()
    }

    pub fn add_black_list(self: &Arc<Self>, remote_addr: String, port: u64) {
        let entry = self
            .black_list
            .write()
            .unwrap()
            .entry(format!("{}:{}", remote_addr, port))
            .or_insert(RwLock::new(ClientInfo {
                remote_addr,
                port: port,
                count: 0,
                last_time: 0,
            }));

        entry.value();
    }
}
