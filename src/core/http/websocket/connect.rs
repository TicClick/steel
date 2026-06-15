use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{client::IntoClientRequest, protocol::WebSocketConfig},
    WebSocketStream,
};
use ureq::http::HeaderValue;

use crate::core::{
    error::SteelApplicationError,
    http::{jwt::AccessTokenTiming, APISettings},
};

async fn connect_with_config(
    request: tokio_tungstenite::tungstenite::http::Request<()>,
    ws_config: WebSocketConfig,
) -> Result<
    (
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        tokio_tungstenite::tungstenite::http::Response<Option<Vec<u8>>>,
    ),
    tokio_tungstenite::tungstenite::Error,
> {
    connect_async_with_config(request, Some(ws_config), true).await
}

async fn wait_until_token_valid(timing: &AccessTokenTiming) {
    let age = timing.age_secs();
    if age < 0.0 {
        log::warn!(
            "Clock skew detected: token appears to be issued {:.2}s in the future (local clock may be behind server time)",
            -age
        );
    }

    let delay_until_valid = timing.delay_until_valid_secs();
    if delay_until_valid > 0.0 {
        log::info!("Token not yet valid, waiting {delay_until_valid:.2}s");
        tokio::time::sleep(std::time::Duration::from_secs_f64(delay_until_valid)).await;
    }
}

pub(super) async fn try_connect_websocket(
    settings: &APISettings,
    token: &str,
    timing: &AccessTokenTiming,
) -> Result<
    (
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        tokio_tungstenite::tungstenite::http::Response<Option<Vec<u8>>>,
    ),
    SteelApplicationError,
> {
    const FRESH_TOKEN_WINDOW: f64 = 2.0;
    const FRESH_TOKEN_RETRIES: u32 = 4;
    const RETRY_DELAY_MS: u64 = 150;

    wait_until_token_valid(timing).await;

    let base_request = settings.ws_base_uri.clone().into_client_request().unwrap();
    let token_header = HeaderValue::from_str(token).unwrap();
    let ws_config = WebSocketConfig::default();

    let max_attempts = if timing.age_secs() < FRESH_TOKEN_WINDOW {
        FRESH_TOKEN_RETRIES
    } else {
        1
    };

    for attempt in 1..=max_attempts {
        if attempt > 1 && timing.age_secs() >= FRESH_TOKEN_WINDOW {
            log::warn!("Token is no longer fresh, stopping retries");
            break;
        }

        let mut request = base_request.clone();
        request
            .headers_mut()
            .insert("Authorization", token_header.clone());

        match connect_with_config(request, ws_config).await {
            Ok((ws, resp)) => {
                log::info!("WebSocket connected on attempt {attempt}/{max_attempts}");
                return Ok((ws, resp));
            }
            Err(e) => {
                let error_msg = e.to_string();
                log::warn!("WebSocket connection attempt {attempt} failed: {error_msg}");

                if super::is_auth_error(&error_msg) && attempt < max_attempts {
                    tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS)).await;
                    continue;
                }
                return Err(SteelApplicationError::InvalidOAuth);
            }
        }
    }

    Err(SteelApplicationError::InvalidOAuth)
}
