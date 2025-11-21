use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 是否启用 TLS
    #[serde(default)]
    pub tls: bool,

    /// TLS 证书路径
    #[serde(default)]
    pub tls_cert: String,

    /// TLS 私钥路径
    #[serde(default)]
    pub tls_key: String,

    /// 监听地址（如 "0.0.0.0:10010"）
    #[serde(default = "default_listening")]
    pub listening: String,

    /// Bearer 认证 Token
    #[serde(default = "generate_token")]
    pub token: String,

    /// HTTP 代理地址（可选）
    #[serde(default)]
    pub http_proxy: String,

    /// 是否跳过上游服务器的 TLS 证书验证（仅用于开发环境）
    #[serde(default)]
    pub insecure_skip_verify: bool,
}

fn default_listening() -> String {
    "0.0.0.0:10010".to_string()
}

fn generate_token() -> String {
    Uuid::new_v4().to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tls: false,
            tls_cert: String::new(),
            tls_key: String::new(),
            listening: default_listening(),
            token: generate_token(),
            http_proxy: String::new(),
            insecure_skip_verify: true,
        }
    }
}

impl Config {
    /// 从文件加载配置
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;

        let config: Config = json5::from_str(&content)
            .with_context(|| "Failed to parse config file")?;

        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .with_context(|| "Failed to serialize config")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {:?}", path.as_ref()))?;

        Ok(())
    }

    /// 创建模板配置文件
    pub fn create_template<P: AsRef<Path>>(path: P) -> Result<()> {
        let config = Config::default();
        config.save_to_file(path)?;
        Ok(())
    }

    /// 加载配置，如果不存在则创建模板
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> Result<Option<Self>> {
        if path.as_ref().exists() {
            let config = Self::load_from_file(&path)?;
            Ok(Some(config))
        } else {
            let template_path = path.as_ref().with_file_name("config.temp.json5");
            Self::create_template(&template_path)?;
            eprintln!("配置文件不存在，已创建模板文件: {:?}", template_path);
            eprintln!("请配置后重命名为 config.json5 并重新运行程序");
            Ok(None)
        }
    }
}
