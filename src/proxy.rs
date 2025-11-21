use crate::headers::{copy_request_headers, copy_response_headers};
use axum::{
    body::Body,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{error, info};
use url::Url;

const PROXY_PATH: &str = "/proxy";

#[derive(Debug, Deserialize)]
pub struct ProxyQuery {
    url: String,
}

pub struct AppState {
    pub client: Client,
}

/// 解析 origin URL（与 Go 版本一致：path 设为 /，清空 query）
fn parse_origin_url(url_string: &str) -> Result<String, url::ParseError> {
    let mut parsed = Url::parse(url_string)?;
    parsed.set_path("/");
    parsed.set_query(None);
    Ok(parsed.to_string().trim_end_matches('/').to_string())
}

/// 构建代理 URL
fn build_proxy_url(uri: &str) -> String {
    format!("{}?url={}", PROXY_PATH, urlencoding::encode(uri))
}

/// 修改 Location 头部（与 Go 版本完全一致）
fn modify_location(response_headers: &mut HeaderMap, origin: &str) {
    // 获取原始 Location
    let raw_location = response_headers
        .get("location")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string());

    let raw_location = match raw_location {
        Some(loc) if !loc.is_empty() => loc,
        _ => return,
    };

    let origin_url = Url::parse(origin).ok();
    let mut location = raw_location.clone();

    // 处理不同类型的 URL
    if location.starts_with("//") {
        // protocol-relative URL
        if let Some(ref url) = origin_url {
            if !url.scheme().is_empty() {
                location = format!("{}:{}", url.scheme(), location);
            }
        }
    } else if !is_full_url(&location) {
        if is_absolute_path(&location) {
            // 绝对路径
            location = format!("{}{}", origin, location);
        } else {
            // 相对路径 like "foo/bar"
            location = format!("{}/{}", origin, location.trim_start_matches('/'));
        }
    }
    // is_full_url 时保持不变

    let location_proxy = build_proxy_url(&location);

    // 删除原始 Location，设置 tun-Location 和 tun-Location-Proxy
    response_headers.remove("location");
    if let Ok(value) = HeaderValue::from_str(&location) {
        response_headers.insert("tun-Location", value);
    }
    if let Ok(value) = HeaderValue::from_str(&location_proxy) {
        response_headers.insert("tun-Location-Proxy", value);
    }
}

fn is_full_url(uri: &str) -> bool {
    uri.starts_with("http://") || uri.starts_with("https://")
}

fn is_absolute_path(uri: &str) -> bool {
    uri.starts_with('/')
}

/// 代理请求处理函数
pub async fn proxy_handler(
    method: Method,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProxyQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AppError> {
    let target_url = &query.url;

    info!("代理请求: {} {}", method, target_url);

    // 解析 origin URL（与 Go 版本一致）
    let origin_url = parse_origin_url(target_url)
        .map_err(|_| AppError::BadRequest("url参数错误".to_string()))?;

    // 复制请求头
    let target_headers = copy_request_headers(&headers)
        .map_err(|e| AppError::Internal(format!("复制请求头失败: {}", e)))?;

    // 转换 HTTP 方法
    let reqwest_method = match method {
        Method::GET => reqwest::Method::GET,
        Method::POST => reqwest::Method::POST,
        Method::PUT => reqwest::Method::PUT,
        Method::DELETE => reqwest::Method::DELETE,
        Method::HEAD => reqwest::Method::HEAD,
        Method::OPTIONS => reqwest::Method::OPTIONS,
        Method::PATCH => reqwest::Method::PATCH,
        _ => reqwest::Method::GET,
    };

    // 构建代理请求
    let mut request_builder = state.client.request(reqwest_method, target_url);

    // 设置请求头
    for (name, value) in target_headers.iter() {
        request_builder = request_builder.header(name, value);
    }

    // 设置请求体（如果有）
    if !body.is_empty() {
        request_builder = request_builder.body(body);
    }

    // 发送请求
    let response = request_builder
        .send()
        .await
        .map_err(|e| {
            error!("{}", e);
            AppError::Internal(e.to_string())
        })?;

    // 获取响应状态码
    let status_code = response.status().as_u16();
    let is_redirect = (300..400).contains(&status_code);

    // 如果是重定向，返回 200 OK，否则返回原状态码
    let final_status = if is_redirect {
        StatusCode::OK
    } else {
        StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK)
    };

    // 复制响应头
    let mut response_headers = HeaderMap::new();
    copy_response_headers(response.headers(), &mut response_headers, status_code);

    // 处理 Location 头部（与 Go 版本一致）
    modify_location(&mut response_headers, &origin_url);

    // 流式传输响应体
    let stream = response.bytes_stream();
    let body = Body::from_stream(stream);

    // 构建响应
    let mut resp = Response::new(body);
    *resp.status_mut() = final_status;
    *resp.headers_mut() = response_headers;

    Ok(resp)
}

/// 添加 CORS 头部（与 Go 版本完全一致）
pub fn add_cors_headers(response_headers: &mut HeaderMap, request_headers: &HeaderMap) {
    // Access-Control-Allow-Origin: 使用请求的 Origin
    let origin = request_headers
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("*");
    if let Ok(value) = HeaderValue::from_str(origin) {
        response_headers.insert("Access-Control-Allow-Origin", value);
    }

    // Access-Control-Allow-Methods: *
    response_headers.insert(
        "Access-Control-Allow-Methods",
        HeaderValue::from_static("*"),
    );

    // Access-Control-Allow-Headers: 使用 access-control-request-headers 或 *
    let request_hdrs = request_headers
        .get("access-control-request-headers")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .unwrap_or("*");
    if let Ok(value) = HeaderValue::from_str(request_hdrs) {
        response_headers.insert("Access-Control-Allow-Headers", value);
    }

    // Access-Control-Max-Age: 86400
    response_headers.insert(
        "Access-Control-Max-Age",
        HeaderValue::from_static("86400"),
    );

    // Access-Control-Allow-Credentials: true
    response_headers.insert(
        "Access-Control-Allow-Credentials",
        HeaderValue::from_static("true"),
    );

    // Access-Control-Expose-Headers（与 Go 版本格式一致）
    response_headers.insert(
        "Access-Control-Expose-Headers",
        HeaderValue::from_static("tun-Location, tun-Location-Proxy, tun-set-cookie, tun-status"),
    );
}

/// 添加缓存控制头部
pub fn add_cache_control_headers(response_headers: &mut HeaderMap) {
    response_headers.insert(
        "Cache-Control",
        HeaderValue::from_static("no-store, no-cache, must-revalidate, post-check=0, pre-check=0"),
    );
    response_headers.insert("Pragma", HeaderValue::from_static("no-cache"));
    response_headers.insert("Expires", HeaderValue::from_static("0"));
}

/// OPTIONS 预检请求处理
pub async fn options_handler(headers: HeaderMap) -> impl IntoResponse {
    let mut response_headers = HeaderMap::new();
    add_cors_headers(&mut response_headers, &headers);
    (StatusCode::OK, response_headers)
}

/// 错误类型定义
#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Internal(String),
    Unauthorized(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
        };

        error!("错误: {} - {}", status, message);

        (status, message).into_response()
    }
}
