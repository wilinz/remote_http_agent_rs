use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

/// Bearer Token 认证中间件
pub async fn auth_middleware(
    State(token): State<Arc<String>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 获取 Authorization 头
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    // 验证 Bearer Token
    if let Some(auth_value) = auth_header {
        if valid_bearer(auth_value, &token) {
            return Ok(next.run(request).await);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

/// 验证 Bearer Token（与 Go 版本完全一致）
pub fn valid_bearer(authorization_header: &str, auth_key: &str) -> bool {
    const BEARER_PREFIX: &str = "Bearer ";

    // 检查是否以 "Bearer " 开头（不区分大小写）
    if !authorization_header
        .to_lowercase()
        .starts_with(BEARER_PREFIX.to_lowercase().as_str())
    {
        return false;
    }

    // 提取 token 并去除空格
    let token = authorization_header[BEARER_PREFIX.len()..].trim();

    // 比较 token
    token == auth_key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_bearer() {
        let token = "test-token-123";

        assert!(valid_bearer("Bearer test-token-123", token));
        assert!(valid_bearer("bearer test-token-123", token));
        assert!(valid_bearer("BEARER test-token-123", token));
        assert!(valid_bearer("Bearer  test-token-123  ", token));

        assert!(!valid_bearer("test-token-123", token));
        assert!(!valid_bearer("Basic test-token-123", token));
        assert!(!valid_bearer("Bearer wrong-token", token));
        assert!(!valid_bearer("", token));
    }
}
