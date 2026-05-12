use axum::http::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashSet;

const TUN_PREFIX: &str = "tun-";

fn default_forward_headers() -> HashSet<String> {
    let mut set = HashSet::new();
    set.insert("content-type".to_string());
    set.insert("content-length".to_string());
    set.insert("referer".to_string());
    set.insert("user-agent".to_string());
    set.insert("accept".to_string());
    set.insert("cookie".to_string());
    set.insert("accept-encoding".to_string());
    set.insert("keep-alive".to_string());
    set
}

fn is_cors_header(header: &str) -> bool {
    header.to_lowercase().starts_with("access-control-")
}

pub fn copy_request_headers(
    source_headers: &HeaderMap,
) -> Result<reqwest::header::HeaderMap, Box<dyn std::error::Error>> {
    let mut target_headers = reqwest::header::HeaderMap::new();
    let whitelist = default_forward_headers();

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

    for (name, value) in source_headers.iter() {
        let name_str = name.as_str();
        let lowered = name_str.to_lowercase();

        let is_tun_header = name_str.len() > TUN_PREFIX.len()
            && name_str[..TUN_PREFIX.len()].eq_ignore_ascii_case(TUN_PREFIX);

        if !is_tun_header && !whitelist.contains(&lowered) {
            continue;
        }

        let new_key = if is_tun_header {
            name_str[TUN_PREFIX.len()..].to_string()
        } else if tun_headers.contains(&lowered) {
            continue;
        } else {
            name_str.to_string()
        };

        if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(new_key.as_bytes()) {
            if let Ok(header_value) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
                target_headers.append(header_name, header_value);
            }
        }
    }

    Ok(target_headers)
}

pub fn copy_response_headers(
    source_headers: &reqwest::header::HeaderMap,
    target_headers: &mut HeaderMap,
    status_code: u16,
) {
    let is_redirect = (300..400).contains(&status_code);

    if is_redirect {
        if let Ok(value) = HeaderValue::from_str(&status_code.to_string()) {
            target_headers.insert("tun-status", value);
        }
    }

    for (name, value) in source_headers.iter() {
        let name_str = name.as_str();

        if is_cors_header(name_str) {
            continue;
        }

        let header_key = if name_str.eq_ignore_ascii_case("set-cookie") {
            "tun-set-cookie"
        } else {
            name_str
        };

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
