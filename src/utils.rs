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
    let mut version = "";
    if let Some(index) = line.find(" ") {
        method = &line[..index];
        line = &line[index + 1..];
    }
    if let Some(index) = line.find(" ") {
        path = &line[..index];
        line = &line[index + 1..];
    }
    version = line;
    headers.insert("method".to_string(), method.to_string());
    headers.insert("path".to_string(), path.to_string());
    headers.insert("version".to_string(), version.to_string());
    for line in lines {
        if line.is_empty() {
            break;
        }
        let mut line = line;
        if let Some(index) = line.find(":") {
            let key = &line[..index];
            line = &line[index + 1..];
            let value = line.trim();
            headers.insert(key.to_string(), value.to_string());
        }
    }
    headers
}
