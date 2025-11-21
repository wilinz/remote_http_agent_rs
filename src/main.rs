mod auth;
mod config;
mod headers;
mod proxy;

use anyhow::{Context, Result};
use axum::{
    body::Body,
    extract::{Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use bytes::Bytes;
use config::Config;
use proxy::{add_cache_control_headers, add_cors_headers, AppState, ProxyQuery};
use std::sync::Arc;

/// 应用配置（包含 token 和 http client）
pub struct AppConfig {
    pub state: Arc<AppState>,
    pub token: String,
}

/// 主处理函数（与 Go 版本的 r.Any("/proxy", ...) 完全一致）
async fn main_handler(
    method: Method,
    State(config): State<Arc<AppConfig>>,
    Query(query): Query<ProxyQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // 1. 先设置 CORS 头部
    let mut response_headers = HeaderMap::new();
    add_cors_headers(&mut response_headers, &headers);

    // 2. 如果是 OPTIONS 请求，直接返回 200（与 Go 版本一致）
    if method == Method::OPTIONS {
        let mut resp = Response::new(Body::empty());
        *resp.status_mut() = StatusCode::OK;
        *resp.headers_mut() = response_headers;
        return resp;
    }

    // 3. 设置缓存控制头部（与 Go 版本一致）
    add_cache_control_headers(&mut response_headers);

    // 4. 验证 Bearer Token（与 Go 版本一致）
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !auth::valid_bearer(auth_header, &config.token) {
        let mut resp = Response::new(Body::from("未认证，请更新App: bearer 认证失败"));
        *resp.status_mut() = StatusCode::UNAUTHORIZED;
        // 合并 CORS 头部到响应
        for (key, value) in response_headers.iter() {
            resp.headers_mut().insert(key, value.clone());
        }
        return resp;
    }

    // 5. 处理代理请求
    match proxy::proxy_handler(method, State(config.state.clone()), Query(query), headers.clone(), body).await {
        Ok(mut resp) => {
            // 合并 CORS 和缓存头部到响应
            for (key, value) in response_headers.iter() {
                resp.headers_mut().insert(key, value.clone());
            }
            resp
        }
        Err(err) => {
            let mut resp = err.into_response();
            // 合并 CORS 头部到错误响应
            for (key, value) in response_headers.iter() {
                resp.headers_mut().insert(key, value.clone());
            }
            resp
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    // 获取当前工作目录（与 Go 版本一致）
    let app_dir = std::env::current_dir()?;
    println!("Current working directory: {:?}", app_dir);

    // 加载配置
    let config = match Config::load_or_create("config.json5")? {
        Some(cfg) => cfg,
        None => {
            // 配置文件不存在，已创建模板
            return Ok(());
        }
    };

    // 构建 HTTP 客户端（与 Go 版本 Transport 配置一致）
    let mut client_builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 分钟超时
        .redirect(reqwest::redirect::Policy::none()) // 不自动跟随重定向
        .danger_accept_invalid_certs(config.insecure_skip_verify); // 跳过上游服务器 TLS 验证

    // 设置代理（与 Go 版本一致）
    if !config.http_proxy.trim().is_empty() {
        match reqwest::Proxy::all(&config.http_proxy) {
            Ok(proxy) => {
                client_builder = client_builder.proxy(proxy);
            }
            Err(e) => {
                println!("http_proxy 格式错误，将使用默认代理: {:?}", e);
            }
        }
    }

    let client = client_builder.build()?;

    // 创建应用配置
    let app_config = Arc::new(AppConfig {
        state: Arc::new(AppState { client }),
        token: config.token.clone(),
    });

    // 构建路由（与 Go 版本 r.Any("/proxy", ...) 一致）
    let app = Router::new()
        .route("/proxy", any(main_handler))
        .with_state(app_config);

    // 启动服务器
    let addr = &config.listening;

    // TLS 支持
    if config.tls {
        // 验证证书和密钥路径
        if config.tls_cert.trim().is_empty() || config.tls_key.trim().is_empty() {
            anyhow::bail!("启用 TLS 时必须提供 tls_cert 和 tls_key 路径");
        }

        println!("运行在 https://{}", addr);
        println!("证书文件: {}", config.tls_cert);
        println!("私钥文件: {}", config.tls_key);

        // 加载 TLS 配置
        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
            &config.tls_cert,
            &config.tls_key,
        )
        .await
        .context("加载 TLS 证书和密钥失败")?;

        // 启动 HTTPS 服务器
        axum_server::bind_rustls(addr.parse()?, tls_config)
            .serve(app.into_make_service())
            .await?;
    } else {
        println!("运行在 http://{}", addr);

        // 启动 HTTP 服务器
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
    }

    Ok(())
}
