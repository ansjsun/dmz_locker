use std::{
    collections::HashMap,
    io::{BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{atomic::Ordering, Arc, RwLock},
    thread::{spawn, JoinHandle},
    time::Duration,
    vec,
};

use log::{error, info};
use moka::sync::Cache;

use crate::{
    domain::{ClientInfo, Config, Mapping},
    utils::{self, current_seconds},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

struct Server {
    servers: RwLock<Vec<JoinHandle<()>>>,
    conns: RwLock<HashMap<String, u64>>,
    white_list: RwLock<Cache<String, u64>>,
    black_list: RwLock<Cache<String, ClientInfo>>,
    errors: Vec<String>,
}

pub fn start(conf: Config) -> Result<()> {
    let server = Arc::new(Server {
        servers: RwLock::new(Default::default()),
        conns: RwLock::new(Default::default()),
        white_list: RwLock::new(
            Cache::builder()
                .time_to_idle(Duration::from_secs(conf.session_time_sec))
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

    for mapping in conf.mappings {
        server.start_listener(mapping)?;
    }

    if server.errors.len() > 0 {
        return Err(server.errors.join("\n").into());
    }

    let listener = TcpListener::bind(format!("0.0.0.0:{}", conf.port)).unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = server.handle_inner(&conf.authentication, stream, conf.port) {
                    error!("handle interal hs error: {}", e);
                }
            }
            Err(e) => return Err(format!("listener on autho server recive error: {}", e).into()),
        }
    }
    Ok(())
}

impl Server {
    fn start_listener(self: &Arc<Self>, mapping: Mapping) -> Result<()> {
        info!("start proxy on :{:?} ", mapping);
        let mapping = Arc::new(mapping);
        let mp = mapping.clone();
        let listener = TcpListener::bind(format!("0.0.0.0:{}", mp.port))?;
        let server = self.clone();
        self.servers.write().unwrap().push(spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let mp = mapping.clone();
                        let server = server.clone();
                        spawn(move || {
                            if let Err(e) = server.handle_proxy(stream, mp) {
                                error!("stream for proxy server recive error: {:?}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("listener on proxy server recive error: {:?}", e);
                    }
                }
            }
            error!("server proxyed on mapping:{:?} shutdownd", mp);
        }));

        Ok(())
    }
}

impl Server {
    fn handle_inner(
        self: &Arc<Self>,
        authentication: &String,
        mut stream: TcpStream,
        port: u16,
    ) -> Result<()> {
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;

        let remote_ip = stream.peer_addr()?.ip().to_string();

        let mut content = String::new();
        let mut reader = BufReader::new(stream.try_clone()?);

        reader.read_to_string(&mut content);

        let head = utils::http_parse(&content);

        if let Some(path) = head.get("path") {
            if "/favicon.ico".eq(path) {
                return Ok(());
            }
            if authentication.eq(path) {
                self.white_list
                    .write()
                    .unwrap()
                    .insert(remote_ip, current_seconds());
            } else {
                self.add_black_list(&remote_ip, port);
                self.white_list.write().unwrap().invalidate(&remote_ip);
                return Err(format!(
                    "remote_ip:{:?} auth error , request path:{:?}",
                    remote_ip, path
                )
                .into());
            }
        }

        let white_list = self
            .white_list
            .read()
            .unwrap()
            .clone()
            .iter()
            .collect::<HashMap<_, _>>();
        let black_list = self
            .black_list
            .read()
            .unwrap()
            .clone()
            .iter()
            .collect::<HashMap<_, _>>();

        let value = serde_json::json!({
            "white_list": white_list,
            "black_list": black_list,
        })
        .to_string();

        let response = b"HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n";
        stream.write(response);
        stream.write(value.as_bytes());
        stream.flush();
        Ok(())
    }

    pub fn handle_proxy(
        self: Arc<Self>,
        mut stream: TcpStream,
        mapping: Arc<Mapping>,
    ) -> Result<()> {
        if !mapping.is_public {
            let remote_ip = stream.peer_addr()?.ip().to_string();

            if !self.white_list.read().unwrap().contains_key(&remote_ip) {
                self.add_black_list(&remote_ip, mapping.port);
                std::thread::sleep(Duration::from_secs(5));
                drop(stream);
                return Ok(());
            }
        }

        let mut target =
            TcpStream::connect_timeout(&mapping.addr.parse()?, Duration::from_secs(5))?;

        let mut stream_write = stream.try_clone()?;
        let mut target_read = target.try_clone()?;

        spawn(move || {
            _ = std::io::copy(&mut stream, &mut target);
        });

        spawn(move || {
            _ = std::io::copy(&mut target_read, &mut stream_write);
        });

        // if mapping.is_public{

        // }
        // let remote_ip = stream.peer_addr()?.ip().to_string();

        Ok(())
    }

    pub fn add_black_list(self: &Arc<Self>, remote_ip: &String, port: u16) {
        info!("add remote_ip :{} on port:{} to blacklist", remote_ip, port);
        let entry = self
            .black_list
            .write()
            .unwrap()
            .entry_by_ref(&format!("{}:{}", remote_ip, port))
            .or_insert(ClientInfo {
                remote_ip: remote_ip.clone(),
                port,
                count: Arc::new(Default::default()),
                last_time: Arc::new(Default::default()),
            });

        let value = entry.value();
        value.last_time.store(current_seconds(), Ordering::SeqCst);
        value.count.fetch_add(1, Ordering::SeqCst);
    }
}
