//! TdPrint 发行版：固定访问令牌。
//!
//! 上游只按 IP 白名单 + Origin 白名单放行，局域网设备（PDA 等 App 内嵌页面的 Origin 是
//! `http://localhost`）和本机调试页面都要逐台配置白名单。TdPrint 额外支持一个编译期注入的
//! 固定令牌：请求携带正确令牌、且 Origin 为空或指向本机（localhost / 回环地址）时，跳过
//! IP 白名单与 Origin 白名单。公网网站即使拿到令牌，Origin 也不是本机，仍然被拒绝。
//!
//! - 令牌在构建时通过环境变量 `TDPRINT_AUTH_TOKEN` 注入（CI 取自仓库 Secret），不写进源码；
//!   未注入时令牌认证整体关闭，行为与上游一致。
//! - 浏览器 WebSocket 不能自定义请求头，令牌放在查询参数：`ws://<ip>:17890/ws?token=<令牌>`；
//!   非浏览器客户端也可以用 `Authorization: Bearer <令牌>`。
//!
//! 这些逻辑集中在本文件，上游文件只保留调用点，便于合并上游更新。

use axum::http::{header::AUTHORIZATION, HeaderMap, Uri};
use std::net::IpAddr;
use url::Url;

/// 令牌查询参数名。
pub const TOKEN_QUERY_PARAM: &str = "token";

/// 编译期注入的固定令牌；未注入或为空时返回 `None`，令牌认证关闭。
pub fn builtin_token() -> Option<&'static str> {
    option_env!("TDPRINT_AUTH_TOKEN")
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

/// 内置令牌是否启用。
pub fn is_enabled() -> bool {
    builtin_token().is_some()
}

/// 请求是否携带了正确的内置令牌（用于跳过 IP 白名单）。
pub fn is_request_authorized(uri: &Uri, headers: &HeaderMap) -> bool {
    token_matches(builtin_token(), uri, headers)
}

/// 携带正确令牌时，Origin 为空或指向本机才放行（用于跳过 Origin 白名单）。
pub fn allows_origin(uri: &Uri, headers: &HeaderMap, origin: Option<&str>) -> bool {
    is_request_authorized(uri, headers) && is_local_origin(origin)
}

/// 比较请求携带的令牌与期望令牌。
fn token_matches(expected: Option<&str>, uri: &Uri, headers: &HeaderMap) -> bool {
    let Some(expected) = expected else {
        return false;
    };
    request_tokens(uri, headers)
        .iter()
        .any(|provided| constant_time_eq(provided.as_bytes(), expected.as_bytes()))
}

/// 取出请求携带的候选令牌：查询参数 `token` 与 `Authorization: Bearer`。
fn request_tokens(uri: &Uri, headers: &HeaderMap) -> Vec<String> {
    let mut tokens = Vec::new();
    if let Some(query) = uri.query() {
        tokens.extend(
            url::form_urlencoded::parse(query.as_bytes())
                .filter(|(key, _)| key == TOKEN_QUERY_PARAM)
                .map(|(_, value)| value.into_owned()),
        );
    }
    if let Some(bearer) = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    {
        tokens.push(bearer.trim().to_string());
    }
    tokens
}

/// Origin 为空（非浏览器客户端）或主机为 localhost / 回环地址。
fn is_local_origin(origin: Option<&str>) -> bool {
    let Some(origin) = origin else {
        return true;
    };
    let Ok(url) = Url::parse(origin) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https" | "capacitor") {
        return false;
    }
    match url.host() {
        Some(url::Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => IpAddr::V4(ip).is_loopback(),
        Some(url::Host::Ipv6(ip)) => IpAddr::V6(ip).is_loopback(),
        None => false,
    }
}

/// 长度不同直接返回 false；长度相同时逐字节比较，不提前退出。
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |diff, (a, b)| diff | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::AgentConfig, state::AgentState};
    use axum::http::{HeaderValue, StatusCode};

    const TOKEN: &str = "s3cret-token";

    fn uri(value: &str) -> Uri {
        value.parse().unwrap()
    }

    #[test]
    fn token_from_query_or_bearer_header_matches() {
        let empty = HeaderMap::new();
        assert!(token_matches(
            Some(TOKEN),
            &uri("/ws?token=s3cret-token"),
            &empty
        ));
        assert!(token_matches(
            Some(TOKEN),
            &uri("/ws?a=1&token=s3cret-token"),
            &empty
        ));

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer s3cret-token"),
        );
        assert!(token_matches(Some(TOKEN), &uri("/ws"), &headers));
    }

    #[test]
    fn wrong_missing_or_disabled_token_does_not_match() {
        let empty = HeaderMap::new();
        assert!(!token_matches(Some(TOKEN), &uri("/ws"), &empty));
        assert!(!token_matches(
            Some(TOKEN),
            &uri("/ws?token=s3cret"),
            &empty
        ));
        assert!(!token_matches(Some(TOKEN), &uri("/ws?token="), &empty));
        assert!(!token_matches(None, &uri("/ws?token=s3cret-token"), &empty));
        assert!(!token_matches(None, &uri("/ws?token="), &empty));
    }

    #[test]
    fn local_origins_are_localhost_or_loopback_only() {
        assert!(is_local_origin(None));
        assert!(is_local_origin(Some("http://localhost")));
        assert!(is_local_origin(Some("https://localhost")));
        assert!(is_local_origin(Some("http://localhost:1024")));
        assert!(is_local_origin(Some("http://127.0.0.1:5173")));
        assert!(is_local_origin(Some("http://[::1]:5173")));
        assert!(is_local_origin(Some("capacitor://localhost")));

        assert!(!is_local_origin(Some("https://oms.tongdacdsc.com")));
        assert!(!is_local_origin(Some("http://localhost.evil.example")));
        assert!(!is_local_origin(Some("http://192.168.1.10:8080")));
        assert!(!is_local_origin(Some("null")));
        assert!(!is_local_origin(Some("file:///tmp/a.html")));
    }

    /// 在回环地址上起真实服务，发 WebSocket 握手，返回 HTTP 状态码（101 为升级成功）。
    async fn handshake_status(path: &str, origin: Option<&str>) -> u16 {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut config = AgentConfig::default();
        config.security.allowed_origins = vec!["https://oms.tongdacdsc.com".to_string()];
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(crate::server::serve_listener(
            AgentState::new(config),
            listener,
        ));

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let origin = origin
            .map(|origin| format!("Origin: {origin}\r\n"))
            .unwrap_or_default();
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n{origin}\r\n"
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut buffer = [0u8; 64];
        let read = stream.read(&mut buffer).await.unwrap();
        let status_line = String::from_utf8_lossy(&buffer[..read]);
        status_line
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap()
    }

    /// 用注入了局域网客户端地址的请求过 IP 白名单中间件；403 表示被 IP 白名单拦截。
    async fn lan_status(path: &str) -> StatusCode {
        use axum::{body::Body, extract::connect_info::ConnectInfo, http::Request};
        use std::net::SocketAddr;
        use tower::ServiceExt;

        let mut request = Request::builder().uri(path).body(Body::empty()).unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([192, 168, 1, 20], 50000))));
        crate::server::router(AgentState::new(AgentConfig::default()))
            .oneshot(request)
            .await
            .unwrap()
            .status()
    }

    fn token_path() -> String {
        format!("/ws?token={}", builtin_token().unwrap_or("not-built-in"))
    }

    #[tokio::test]
    async fn allowlisted_origin_still_works_without_token() {
        assert_eq!(
            handshake_status("/ws", Some("https://oms.tongdacdsc.com")).await,
            101
        );
    }

    #[tokio::test]
    async fn local_origin_without_token_is_rejected() {
        assert_eq!(
            handshake_status("/ws", Some("http://localhost:1024")).await,
            403
        );
        assert_eq!(
            handshake_status("/ws?token=wrong", Some("http://localhost")).await,
            403
        );
        assert_eq!(handshake_status("/ws", None).await, 403);
        assert_eq!(lan_status("/ws?token=wrong").await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn builtin_token_allows_local_origins_and_lan_clients() {
        let path = token_path();
        // 未注入令牌时（本地开发构建）行为与上游一致。
        let local = if is_enabled() { 101 } else { 403 };
        assert_eq!(
            handshake_status(&path, Some("http://localhost")).await,
            local
        );
        assert_eq!(
            handshake_status(&path, Some("http://127.0.0.1:5173")).await,
            local
        );
        assert_eq!(handshake_status(&path, None).await, local);
        assert_eq!(
            lan_status(&path).await == StatusCode::FORBIDDEN,
            !is_enabled(),
            "LAN client with token"
        );
    }

    #[tokio::test]
    async fn builtin_token_does_not_unlock_public_origins() {
        assert_eq!(
            handshake_status(&token_path(), Some("https://evil.example")).await,
            403
        );
    }

    #[test]
    fn constant_time_eq_compares_whole_value() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}
