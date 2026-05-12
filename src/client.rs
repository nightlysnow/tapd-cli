use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use base64::Engine;
use reqwest::blocking::Client;
use serde_json::{json, Map, Value};

use crate::config::Config;

pub struct TapdClient {
    http: Client,
    base_url: String,
    headers: reqwest::header::HeaderMap,
    is_cloud: bool,
    pub nick: Option<String>,
    pub bot_url: Option<String>,
    #[allow(dead_code)]
    pub web_base_url: Option<String>,
}

impl TapdClient {
    pub fn new(config: &Config) -> Result<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse()?);
        headers.insert("Via", "mcp".parse()?);

        if let Some(token) = &config.access_token {
            headers.insert("Authorization", format!("Bearer {token}").parse()?);
        } else if let (Some(user), Some(pass)) = (&config.api_user, &config.api_password) {
            let encoded =
                base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}"));
            headers.insert("Authorization", format!("Basic {encoded}").parse()?);
        }

        let mut client = Self {
            http,
            base_url: config.api_base_url.clone(),
            headers,
            is_cloud: config.is_cloud(),
            nick: config.nick.clone(),
            bot_url: config.bot_url.clone(),
            web_base_url: config.web_base_url.clone(),
        };

        if client.nick.is_none() {
            let auth_hint = if config.access_token.is_some() {
                "TAPD_ACCESS_TOKEN"
            } else {
                "TAPD_API_USER / TAPD_API_PASSWORD"
            };
            match client.get("users/info", &HashMap::new()) {
                Ok(resp) => {
                    client.nick = resp
                        .get("data")
                        .and_then(|d| d.get("nick"))
                        .and_then(|n| n.as_str())
                        .map(String::from);
                }
                Err(e) => {
                    if e.to_string().contains("认证失败") {
                        bail!("凭证无效或已过期，请检查 {auth_hint} 配置");
                    }
                }
            }
        }

        Ok(client)
    }

    pub fn get(&self, endpoint: &str, params: &HashMap<String, String>) -> Result<Value> {
        let url = self.url(endpoint);
        let resp = self
            .http
            .get(&url)
            .headers(self.headers.clone())
            .query(params)
            .send()
            .context("HTTP 请求失败")?;

        Self::parse_response(resp)
    }

    pub fn post(&self, endpoint: &str, data: &Value) -> Result<Value> {
        let url = self.url(endpoint);
        let resp = self
            .http
            .post(&url)
            .headers(self.headers.clone())
            .json(data)
            .send()
            .context("HTTP 请求失败")?;

        Self::parse_response(resp)
    }

    pub fn send_wecom(&self, msg: &str) -> Result<Value> {
        let bot_url = self.bot_url.as_deref().context("BOT_URL 未配置")?;
        let body = if msg.contains('@') {
            json!({ "msgtype": "markdown", "markdown": { "content": msg } })
        } else {
            json!({ "msgtype": "markdown_v2", "markdown_v2": { "content": msg } })
        };
        let resp = self
            .http
            .post(bot_url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .context("发送企业微信消息失败")?;
        let text = resp.text()?;
        Ok(serde_json::from_str(&text).unwrap_or_else(|_| json!({ "response": text })))
    }

    /// Normalize short IDs (≤9 digits) to TAPD long-ID format.
    pub fn normalize_ids(&self, params: &mut HashMap<String, String>) {
        let ws = match params.get("workspace_id") {
            Some(w) => w.clone(),
            None => return,
        };
        for key in ["id", "entry_id", "object_id", "story_id"] {
            if let Some(val) = params.get(key).cloned() {
                params.insert(key.into(), self.to_long_ids(&val, &ws));
            }
        }
    }

    fn to_long_ids(&self, id_str: &str, workspace_id: &str) -> String {
        let prefix = if self.is_cloud { "11" } else { "10" };
        let convert = |s: &str| {
            let s = s.trim();
            if s.len() <= 9 && s.chars().all(|c| c.is_ascii_digit()) {
                format!("{prefix}{workspace_id}{s:0>9}")
            } else {
                s.to_string()
            }
        };

        if id_str.contains(',') {
            id_str.split(',').map(convert).collect::<Vec<_>>().join(",")
        } else {
            convert(id_str)
        }
    }

    /// Set nick-derived field only if not already provided.
    pub fn set_nick_default(&self, params: &mut HashMap<String, String>, field: &str) {
        if let Some(nick) = &self.nick {
            params.entry(field.into()).or_insert_with(|| nick.clone());
        }
    }

    /// Always override the nick-derived field (used for updates).
    pub fn set_nick_override(&self, params: &mut HashMap<String, String>, field: &str) {
        if let Some(nick) = &self.nick {
            params.insert(field.into(), nick.clone());
        }
    }

    fn url(&self, endpoint: &str) -> String {
        let base = format!("{}/{endpoint}", self.base_url);
        let sep = if base.contains('?') { '&' } else { '?' };
        format!("{base}{sep}s=mcp")
    }

    fn parse_response(resp: reqwest::blocking::Response) -> Result<Value> {
        let status = resp.status();
        let body = resp.text().context("读取响应体失败")?;
        if !status.is_success() {
            if status.as_u16() == 401 || status.as_u16() == 422 {
                if body.contains("invalid") || body.contains("unauthorized") {
                    bail!(
                        "认证失败: 凭证无效或已过期，请检查 .env 中的认证配置"
                    );
                }
            }
            bail!("API 错误 ({status}): {body}");
        }
        serde_json::from_str(&body).context("JSON 解析失败")
    }
}

/// Parse `["key=value", ...]` into a HashMap.
pub fn parse_params(raw: &[String]) -> HashMap<String, String> {
    raw.iter()
        .filter_map(|s| s.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Build a params map with workspace_id and sensible defaults.
pub fn with_workspace(
    ws: u64,
    raw: &[String],
    defaults: &[(&str, &str)],
) -> HashMap<String, String> {
    let mut map = parse_params(raw);
    map.insert("workspace_id".into(), ws.to_string());
    for &(k, v) in defaults {
        map.entry(k.into()).or_insert_with(|| v.into());
    }
    map
}

/// Convert string params to a JSON Value, coercing numeric strings.
pub fn to_json(params: &HashMap<String, String>) -> Value {
    let map: Map<String, Value> = params
        .iter()
        .map(|(k, v)| {
            let val = v
                .parse::<i64>()
                .map(|n| Value::Number(n.into()))
                .unwrap_or_else(|_| Value::String(v.clone()));
            (k.clone(), val)
        })
        .collect();
    Value::Object(map)
}
