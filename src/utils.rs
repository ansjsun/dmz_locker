use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn current_seconds() -> u64 {
    let now = SystemTime::now();
    let since_the_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    since_the_epoch.as_secs()
}

pub(crate) fn http_parse(content: &str) -> HashMap<String, String> {
    //parse http header
    let mut headers = HashMap::new();
    let mut lines = content.lines();
    let mut line = lines.next().unwrap_or_default();
    let mut method = "";
    let mut path = "";
    if let Some(index) = line.find(" ") {
        method = &line[..index];
        line = &line[index + 1..];
    }
    if let Some(index) = line.find(" ") {
        path = &line[..index];
    }
    headers.insert("method".to_string(), method.to_string());
    headers.insert("path".to_string(), path.to_string());
    headers
}
