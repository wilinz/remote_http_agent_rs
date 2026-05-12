#![cfg_attr(all(windows, feature = "gui"), windows_subsystem = "windows")]

mod auth;
mod config;
mod headers;
mod ip;
mod proxy;

use anyhow::Result;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::Response,
    routing::{any, get},
    Router,
};
use config::Config;
use proxy::{add_cache_control_headers, add_cors_headers, AppState};
use reqwest::Client;
use std::sync::Arc;

pub struct AppConfig {
    pub state: Arc<AppState>,
    pub token: String,
}


async fn app_middleware(
    State(config): State<Arc<AppConfig>>,
    request: Request,
    next: Next,
) -> Response {
    let request_headers = request.headers().clone();
    let method = request.method().clone();

    let mut cors_headers = HeaderMap::new();
    add_cors_headers(&mut cors_headers, &request_headers);

    // OPTIONS 直接返回 204，不做认证（与 Go 版本一致）
    if method == Method::OPTIONS {
        let mut resp = Response::new(Body::empty());
        *resp.status_mut() = StatusCode::NO_CONTENT;
        *resp.headers_mut() = cors_headers;
        return resp;
    }

    add_cache_control_headers(&mut cors_headers);

    let auth_header = request_headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !auth::valid_bearer(auth_header, &config.token) {
        let body =
            serde_json::json!({"error": "未认证，请更新App: bearer 认证失败"}).to_string();
        let mut resp = Response::new(Body::from(body));
        *resp.status_mut() = StatusCode::UNAUTHORIZED;
        resp.headers_mut().insert(
            "content-type",
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        for (k, v) in cors_headers.iter() {
            resp.headers_mut().insert(k, v.clone());
        }
        return resp;
    }

    let mut resp = next.run(request).await;
    for (k, v) in cors_headers.iter() {
        resp.headers_mut().insert(k, v.clone());
    }
    resp
}

async fn kill_handler() -> impl axum::response::IntoResponse {
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        std::process::exit(0);
    });
    axum::Json(serde_json::json!({"code": 0, "msg": "程序即将退出"}))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let app_dir = std::env::current_dir()?;
    println!("Current working directory: {:?}", app_dir);

    let pid = std::process::id();
    #[cfg(windows)]
    {
        let kill_bat_content = format!(
            "@echo off\r\ntaskkill /PID {} /F 2>nul\r\necho Service stopped\r\n",
            pid
        );
        let kill_bat_path = app_dir.join("kill.bat");
        if let Err(e) = std::fs::write(&kill_bat_path, kill_bat_content) {
            eprintln!("生成 kill.bat 失败: {}", e);
        } else {
            println!("已生成 kill.bat");
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let kill_sh_content = format!("#!/bin/sh\nkill {} 2>/dev/null\necho 'Service stopped'\n", pid);
        let kill_sh_path = app_dir.join("kill.sh");
        if let Err(e) = std::fs::write(&kill_sh_path, &kill_sh_content) {
            eprintln!("生成 kill.sh 失败: {}", e);
        } else {
            let _ = std::fs::set_permissions(&kill_sh_path, std::fs::Permissions::from_mode(0o755));
            println!("已生成 kill.sh");
        }
    }

    let config = Config::load_or_create("config.json5")?;

    let mut client_builder = Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .redirect(reqwest::redirect::Policy::none())
        .danger_accept_invalid_certs(config.skip_tls);

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

    let app_config = Arc::new(AppConfig {
        state: Arc::new(AppState { client }),
        token: config.token.clone(),
    });

    let app = Router::new()
        .route("/proxy", any(proxy::proxy_request_handler))
        .route("/lanip", get(ip::get_lan_ip_handler))
        .route("/kill", get(kill_handler))
        .layer(axum::middleware::from_fn_with_state(
            app_config.clone(),
            app_middleware,
        ))
        .with_state(app_config);

    let addr = &config.listening;
    println!("运行在 http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
