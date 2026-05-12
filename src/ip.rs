use axum::{http::StatusCode, response::IntoResponse, Json};
use if_addrs::{get_if_addrs, IfAddr};
use serde_json::json;

pub fn get_local_ip() -> Result<String, String> {
    let interfaces = get_if_addrs().map_err(|e| e.to_string())?;

    let mut ethernet_ip: Option<String> = None;
    let mut wifi_ip: Option<String> = None;
    let mut fallback_ip: Option<String> = None;

    for iface in interfaces {
        if iface.is_loopback() {
            continue;
        }

        let ip = match iface.addr {
            IfAddr::V4(ref v4) => v4.ip,
            _ => continue,
        };

        if ip.is_loopback() || ip.octets()[0] == 127 {
            continue;
        }

        let ip_str = ip.to_string();
        let name = iface.name.to_lowercase();

        let is_ethernet = name.contains("eth")
            || name.contains("en")
            || name.contains("ethernet")
            || name.contains("以太网")
            || name.contains("本地连接")
            || (name.starts_with("en") && name.len() <= 4);

        let is_wifi = name.contains("wlan")
            || name.contains("wifi")
            || name.contains("wi-fi")
            || name.contains("wl")
            || name.contains("无线")
            || name.contains("wireless");

        if is_ethernet && ethernet_ip.is_none() {
            ethernet_ip = Some(ip_str);
        } else if is_wifi && wifi_ip.is_none() {
            wifi_ip = Some(ip_str);
        } else if fallback_ip.is_none() {
            fallback_ip = Some(ip_str);
        }
    }

    ethernet_ip
        .or(wifi_ip)
        .or(fallback_ip)
        .ok_or_else(|| "未找到可用的 IPv4 地址".to_string())
}

pub async fn get_lan_ip_handler() -> impl IntoResponse {
    match get_local_ip() {
        Ok(ip) => (
            StatusCode::OK,
            Json(json!({"code": 0, "msg": "success", "ip": ip})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"code": -1, "msg": e, "ip": ""})),
        )
            .into_response(),
    }
}
