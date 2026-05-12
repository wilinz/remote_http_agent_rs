/// 验证 Bearer Token（与 Go 版本完全一致）
pub fn valid_bearer(authorization_header: &str, auth_key: &str) -> bool {
    const BEARER_PREFIX: &str = "Bearer ";

    if !authorization_header
        .to_lowercase()
        .starts_with(BEARER_PREFIX.to_lowercase().as_str())
    {
        return false;
    }

    let token = authorization_header[BEARER_PREFIX.len()..].trim();
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
