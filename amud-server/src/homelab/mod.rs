mod ai;
mod apps;
mod health;
mod media;
mod monitoring;
mod network;
mod tier2;

pub use ai::*;
pub use apps::*;
pub use health::{fetch_health_integration, is_health_only, HEALTH_ONLY_TYPES};
pub use media::*;
pub use monitoring::*;
pub use network::*;
pub use tier2::*;

use reqwest::Client;
use serde_json::Value;

pub(crate) fn parse_pipe_credential(raw: &str) -> Option<(&str, &str)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "none" {
        return None;
    }
    let (left, right) = trimmed.split_once('|')?;
    let left = left.trim();
    let right = right.trim();
    if left.is_empty() || right.is_empty() {
        return None;
    }
    Some((left, right))
}

pub(crate) fn opnsense_basic_auth(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some((key, secret)) = parse_pipe_credential(trimmed) {
        use base64::Engine;
        return base64::engine::general_purpose::STANDARD.encode(format!("{key}:{secret}"));
    }
    trimmed.to_string()
}

pub(crate) fn json_u64(v: &Value, keys: &[&str]) -> u64 {
    keys.iter()
        .find_map(|k| {
            v.get(*k).and_then(|x| {
                x.as_u64()
                    .or_else(|| x.as_i64().map(|n| n.max(0) as u64))
                    .or_else(|| x.as_f64().map(|n| n.max(0.0) as u64))
            })
        })
        .unwrap_or(0)
}

pub(crate) fn json_str<'a>(v: &'a Value, keys: &[&str]) -> &'a str {
    keys.iter()
        .find_map(|k| v.get(*k).and_then(|x| x.as_str()))
        .unwrap_or("—")
}

pub(crate) fn auth_header(token: &str) -> String {
    if token.starts_with("Bearer ") || token.starts_with("Token ") {
        token.to_string()
    } else {
        format!("Bearer {token}")
    }
}

pub(crate) fn basic_auth(user: &str, pass: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}"))
}

pub(crate) async fn get_json(client: &Client, url: &str, auth: Option<&str>) -> Option<Value> {
    let mut req = client.get(url);
    if let Some(a) = auth {
        let prefixed = a.starts_with("PVEAPIToken=")
            || a.starts_with("PBSAPIToken=")
            || a.starts_with("Basic ")
            || a.starts_with("Bearer ")
            || a.starts_with("Token ")
            || a.starts_with("api-key ");
        if a.contains(':') && !prefixed {
            req = req.header("Authorization", format!("Basic {a}"));
        } else {
            req = req.header("Authorization", a);
        }
    }
    let resp = req.send().await.ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub(crate) async fn post_json(
    client: &Client,
    url: &str,
    body: &Value,
    auth: Option<&str>,
) -> Option<Value> {
    let mut req = client.post(url).json(body);
    if let Some(a) = auth {
        req = req.header("Authorization", a);
    }
    let resp = req.send().await.ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub fn build_homelab_client(clients: &crate::http_client::SharedHttpClients) -> Client {
    clients.homelab.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_pipe_credential_splits_user_pass() {
        let (u, p) = parse_pipe_credential("admin|secret").unwrap();
        assert_eq!(u, "admin");
        assert_eq!(p, "secret");
        assert!(parse_pipe_credential("bad").is_none());
        assert!(parse_pipe_credential("none").is_none());
    }

    #[test]
    fn opnsense_basic_auth_encodes_key_secret() {
        use base64::Engine;
        let encoded = opnsense_basic_auth("mykey|mysecret");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        assert_eq!(decoded, b"mykey:mysecret");
    }

    #[test]
    fn parse_glances_quicklook_extracts_cpu_mem() {
        let v = json!({ "cpu": { "total": 43.0 }, "mem": { "percent": 68.0 }, "load": { "min1": 1.25 } });
        let out = monitoring::parse_glances_quicklook(&v);
        assert_eq!(out["cpu"], "43%");
        assert_eq!(out["memory"], "68%");
        assert_eq!(out["type"], "glances");
    }
}
