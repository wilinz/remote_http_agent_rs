use axum::http::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashSet;

const TUN_PREFIX: &str = "tun-";

/// 默认转发的头部白名单（与 Go 版本完全一致）
fn default_forward_headers() -> HashSet<String> {
    let mut set = HashSet::new();
    set.insert("content-type".to_string());
    set.insert("content-length".to_string());
    set.insert("user-agent".to_string());
    set.insert("accept".to_string());
    set.insert("accept-encoding".to_string());
    set.insert("keep-alive".to_string());
    set
}

/// 判断是否为 CORS 头部
fn is_cors_header(header: &str) -> bool {
    header.to_lowercase().starts_with("access-control-")
}

/// 复制请求头到目标请求（与 Go 版本 copyRequestHeader 完全一致）
pub fn copy_request_headers(
    source_headers: &HeaderMap,
) -> Result<reqwest::header::HeaderMap, Box<dyn std::error::Error>> {
    let mut target_headers = reqwest::header::HeaderMap::new();
    let whitelist = default_forward_headers();

    // 收集所有 tun- 前缀的头部（与 Go 版本一致）
    let mut tun_headers = HashSet::new();
    for (name, _) in source_headers.iter() {
        let name_str = name.as_str();
        if name_str.len() > TUN_PREFIX.len()
            && name_str[..TUN_PREFIX.len()].eq_ignore_ascii_case(TUN_PREFIX)
        {
            let original_name = name_str[TUN_PREFIX.len()..].to_lowercase();
            tun_headers.insert(original_name);
        }
    }

    // 处理所有头部
    for (name, value) in source_headers.iter() {
        let name_str = name.as_str();
        let lowered = name_str.to_lowercase();

        // 检查是否是 tun- 前缀的头部
        let is_tun_header = name_str.len() > TUN_PREFIX.len()
            && name_str[..TUN_PREFIX.len()].eq_ignore_ascii_case(TUN_PREFIX);

        // 如果不是 tun- 头部，也不在白名单中，跳过
        if !is_tun_header && !whitelist.contains(&lowered) {
            continue;
        }

        let new_key = if is_tun_header {
            // 去除 tun- 前缀
            name_str[TUN_PREFIX.len()..].to_string()
        } else if tun_headers.contains(&lowered) {
            // 白名单头部但存在 tun- 版本，跳过以避免重复
            continue;
        } else {
            name_str.to_string()
        };

        // 添加头部（使用 append 支持多个值，与 Go 的 Add 一致）
        if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(new_key.as_bytes()) {
            if let Ok(header_value) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
                target_headers.append(header_name, header_value);
            }
        }
    }

    Ok(target_headers)
}

/// 复制响应头到代理响应（与 Go 版本 copyResponseHeader 完全一致）
pub fn copy_response_headers(
    source_headers: &reqwest::header::HeaderMap,
    target_headers: &mut HeaderMap,
    status_code: u16,
) {
    let is_redirect = (300..400).contains(&status_code);

    // 如果是重定向，记录原始状态码
    if is_redirect {
        if let Ok(value) = HeaderValue::from_str(&status_code.to_string()) {
            target_headers.insert("tun-status", value);
        }
    }

    // 遍历所有响应头
    for (name, value) in source_headers.iter() {
        let name_str = name.as_str();

        // 跳过 CORS 头部（与 Go 版本一致）
        if is_cors_header(name_str) {
            continue;
        }

        // Set-Cookie 重命名为 tun-set-cookie（与 Go 版本一致）
        let header_key = if name_str.eq_ignore_ascii_case("set-cookie") {
            "tun-set-cookie"
        } else {
            name_str
        };

        // 添加头部（使用 append 支持多个值，与 Go 的 Add 一致）
        if let Ok(header_name) = HeaderName::try_from(header_key) {
            if let Ok(header_value) = HeaderValue::try_from(value.as_bytes()) {
                target_headers.append(header_name, header_value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cors_header() {
        assert!(is_cors_header("Access-Control-Allow-Origin"));
        assert!(is_cors_header("access-control-allow-methods"));
        assert!(!is_cors_header("Content-Type"));
        assert!(!is_cors_header("X-Custom-Header"));
    }
}
