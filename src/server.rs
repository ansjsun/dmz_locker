use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{atomic::Ordering, Arc, RwLock},
    time::Duration,
    vec,
};

use log::{error, info};
use moka::sync::Cache;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
    try_join,
};

use crate::{
    domain::{ClientInfo, Config, Mapping},
    utils::{self, current_seconds},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

struct Server {
    conns: RwLock<HashMap<String, u64>>,
    white_list: RwLock<Cache<String, u64>>,
    black_list: RwLock<Cache<String, ClientInfo>>,
    errors: Vec<String>,
}

pub async fn start(conf: Config) -> Result<()> {
    let server = Arc::new(Server {
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
        server.start_listener(mapping).await?;
    }

    if server.errors.len() > 0 {
        return Err(server.errors.join("\n").into());
    }

    let listener = TcpListener::bind(format!("0.0.0.0:{}", conf.port))
        .await
        .unwrap();

    loop {
        let stream = listener.accept().await;
        match stream {
            Ok((stream, addr)) => {
                let authentication = conf.authentication.clone();
                let server = server.clone();
                tokio::spawn(async move {
                    if let Err(e) = server
                        .handle_inner(authentication, stream, addr, conf.port)
                        .await
                    {
                        error!("handle interal hs error: {}", e);
                    }
                });
            }
            Err(e) => return Err(format!("listener on autho server recive error: {}", e).into()),
        }
    }
}

impl Server {
    async fn start_listener(self: &Arc<Self>, mapping: Mapping) -> Result<()> {
        info!("start proxy on :{:?} ", mapping);
        let mapping = Arc::new(mapping);
        let mp = mapping.clone();
        let listener = TcpListener::bind(format!("0.0.0.0:{}", mp.port)).await?;

        let server = self.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let mp = mapping.clone();
                        let server = server.clone();
                        tokio::spawn(async move {
                            if let Err(e) = server.handle_proxy(stream, addr, mp).await {
                                error!("stream for proxy server recive error: {:?}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("listener on proxy server recive error: {:?}", e);
                    }
                }
            }
        });

        Ok(())
    }
}

impl Server {
    async fn handle_inner(
        self: &Arc<Self>,
        authentication: String,
        mut stream: TcpStream,
        addr: SocketAddr,
        port: u16,
    ) -> Result<()> {
        let _ = stream.set_nodelay(false);

        let remote_ip = addr.ip().to_string();

        let mut buffer = [0u8; 512];

        let n = timeout(Duration::from_secs(30), stream.read(&mut buffer)).await??;

        let head = utils::http_parse(&String::from_utf8_lossy(&buffer[..n]));

        if let Some(path) = head.get("path") {
            if "/favicon.ico".eq(path) {
                return Ok(());
            }
            if authentication.eq(path) {
                info!("success for ip :{} add to white list", remote_ip);
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

        let conns = self.conns.read().unwrap().clone();

        let value = serde_json::json!({
            "white_list": white_list,
            "black_list": black_list,
            "conns":conns,
        })
        .to_string();

        let response = format!(
            r#"HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Content-Length: {}
Cache-Control: no-cache
Content-Type: application/json;charset=utf-8

{}"#,
            value.len(),
            value
        );

        stream.write_all(response.as_bytes()).await?;
        stream.flush().await?;
        Ok(())
    }

    pub async fn handle_proxy(
        self: Arc<Self>,
        mut stream: TcpStream,
        addr: SocketAddr,
        mapping: Arc<Mapping>,
    ) -> Result<()> {
        let remote_ip = addr.ip().to_string();
        let remote_port = addr.port();
        if !mapping.is_public {
            if !self.white_list.read().unwrap().contains_key(&remote_ip) {
                self.add_black_list(&remote_ip, mapping.port);
                std::thread::sleep(Duration::from_secs(5));
                drop(stream);
                return Ok(());
            }
        }

        let mut target = TcpStream::connect(mapping.addr.as_str()).await?;

        let (mut ir, mut iw) = stream.split();

        let (mut sr, mut sw) = target.split();

        let s1 = tokio::io::copy(&mut ir, &mut sw);
        let s2 = tokio::io::copy(&mut sr, &mut iw);

        let key = format!("{}:{}->{:?}", remote_ip, remote_port, mapping.addr);
        self.conns
            .write()
            .unwrap()
            .insert(key.clone(), current_seconds());
        let _ = try_join!(s1, s2);
        self.conns.write().unwrap().remove(&key);

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
