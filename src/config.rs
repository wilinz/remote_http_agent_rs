use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 监听地址（如 "0.0.0.0:10010"）
    #[serde(default = "default_listening")]
    pub listening: String,

    /// Bearer 认证 Token
    #[serde(default = "default_token")]
    pub token: String,

    /// HTTP 代理地址（可选）
    #[serde(default = "default_http_proxy")]
    pub http_proxy: String,

    /// 是否跳过上游服务器的 TLS 证书验证
    #[serde(default = "default_skip_tls")]
    pub skip_tls: bool,
}

fn default_listening() -> String {
    option_env!("DEFAULT_LISTENING")
        .unwrap_or("0.0.0.0:10010")
        .to_string()
}

fn default_token() -> String {
    option_env!("DEFAULT_TOKEN")
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

fn default_skip_tls() -> bool {
    option_env!("DEFAULT_SKIP_TLS")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(true)
}

fn default_http_proxy() -> String {
    option_env!("DEFAULT_HTTP_PROXY")
        .unwrap_or("")
        .to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listening: default_listening(),
            token: default_token(),
            http_proxy: default_http_proxy(),
            skip_tls: default_skip_tls(),
        }
    }
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;

        let config: Config = json5::from_str(&content)
            .with_context(|| "Failed to parse config file")?;

        Ok(config)
    }

    /// 加载配置，如果不存在则使用默认值（与 Go 版本一致）
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> Result<Self> {
        if path.as_ref().exists() {
            let config = Self::load_from_file(&path)?;
            println!("已加载配置文件: {:?}", path.as_ref().file_name().unwrap_or_default());
            Ok(config)
        } else {
            println!("配置文件不存在，使用默认配置");
            Ok(Config::default())
        }
    }
}
