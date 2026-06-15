use steel_core::chat::ConnectionStatus;
use steel_core::ipc::server::AppMessageIn;
use tokio::sync::mpsc::UnboundedSender;

use crate::core::{
    error::{SteelApplicationError, SteelApplicationResult},
    http::{
        api::Client, send_progress, token_refresh::TokenRefreshError, token_storage, APISettings,
    },
};

use super::connect::try_connect_websocket;
use super::session::{run_session, SessionEnd};

pub async fn websocket_thread_main(
    tx: UnboundedSender<AppMessageIn>,
    settings: APISettings,
    client: Client,
) -> SteelApplicationResult<()> {
    let send_disconnect = |tx: &UnboundedSender<AppMessageIn>, by_user: bool, auth_failed: bool| {
        tx.send(AppMessageIn::connection_changed(
            ConnectionStatus::Disconnected {
                by_user,
                auth_failed,
            },
        ))
        .unwrap_or_else(|e| log::error!("Failed to send disconnect: {e}"));
    };

    let abort_to_login = |message: String, clear_tokens: bool| {
        if clear_tokens {
            if let Err(e) = token_storage::clear_token_state() {
                log::error!("Failed to clear token state: {e}");
            }
        }
        tx.send(AppMessageIn::ui_show_error(
            Box::new(std::io::Error::other(message)),
            false,
        ))
        .unwrap_or_else(|e| log::error!("Failed to send error: {e}"));
        send_disconnect(&tx, false, true);
    };

    const SESSION_INVALID_MESSAGE: &str =
        "Your osu! session is no longer valid - please log in again";

    let mut refresh_attempted = false;

    loop {
        let Some(access_token) = client.get_access_token().await else {
            log::warn!("API client does not have an access token");
            send_disconnect(&tx, false, true);
            return Err(SteelApplicationError::InvalidOAuth);
        };

        let Some(timing) = client.access_token_timing() else {
            log::error!("Failed to parse the access token, cannot proceed with connection");
            send_disconnect(&tx, false, true);
            return Err(SteelApplicationError::InvalidOAuth);
        };
        log::debug!(
            "Access token timing: iat={}, nbf={}, exp={}",
            timing.issued_at,
            timing.not_before,
            timing.expires_at
        );

        send_progress(&tx, "connecting to the chat server");

        let session_end = match try_connect_websocket(&settings, &access_token, &timing).await {
            Ok((ws, resp)) => {
                log::debug!("WS: Connected with: {resp:?}");
                run_session(ws, &tx, &settings, &client, &mut refresh_attempted).await
            }
            Err(e) => {
                log::error!("Failed to connect to WebSocket: {:?}", e);
                SessionEnd::AuthFailure
            }
        };

        match session_end {
            SessionEnd::Shutdown => {
                send_disconnect(&tx, true, false);
                return Ok(());
            }
            SessionEnd::ConnectionLost => {
                send_disconnect(&tx, false, false);
                return Ok(());
            }
            SessionEnd::AuthFailure => {
                if refresh_attempted {
                    log::warn!(
                        "The token obtained moments ago has been rejected as well - login required"
                    );
                    abort_to_login(SESSION_INVALID_MESSAGE.to_owned(), true);
                    return Err(SteelApplicationError::InvalidOAuth);
                }
                refresh_attempted = true;

                if !client.is_refresh_token_usable() {
                    log::warn!(
                        "Authentication failed and there is no usable refresh token - login required"
                    );
                    abort_to_login(SESSION_INVALID_MESSAGE.to_owned(), true);
                    return Err(SteelApplicationError::InvalidOAuth);
                }

                log::info!("Authentication failed - refreshing the token and reconnecting");
                tx.send(AppMessageIn::connection_changed(
                    ConnectionStatus::InProgress,
                ))
                .unwrap_or_else(|e| log::error!("Failed to send connection status: {e}"));
                send_progress(
                    &tx,
                    "the server rejected the token, falling back to a refresh",
                );

                if let Err(e) = client.refresh_token_now().await {
                    log::error!("Token refresh failed: {e}");
                    let rejected = e
                        .downcast_ref::<TokenRefreshError>()
                        .is_some_and(|e| matches!(e, TokenRefreshError::Rejected(_)));
                    if rejected {
                        abort_to_login(SESSION_INVALID_MESSAGE.to_owned(), true);
                    } else {
                        abort_to_login(format!("Failed to refresh the API token: {e}"), false);
                    }
                    return Err(SteelApplicationError::InvalidOAuth);
                }
            }
        }
    }
}
