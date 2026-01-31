use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use steel_core::ipc::server::{AppMessageIn, OAuthTokens};
use steel_core::settings::chat::OAuthMode;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
pub struct OAuthListenerConfig {
    pub mode: OAuthMode,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug)]
pub enum OAuthListenerError {
    BindFailed(std::io::Error),
    InvalidRequest(String),
    MissingParameter(String),
    AuthFailed(String),
    TokenExchangeFailed(String),
}

impl std::fmt::Display for OAuthListenerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthListenerError::BindFailed(e) => write!(f, "Failed to bind to port: {}", e),
            OAuthListenerError::InvalidRequest(s) => write!(f, "Invalid request: {}", s),
            OAuthListenerError::MissingParameter(s) => write!(f, "Missing parameter: {}", s),
            OAuthListenerError::AuthFailed(s) => write!(f, "Authentication failed: {}", s),
            OAuthListenerError::TokenExchangeFailed(s) => {
                write!(f, "Token exchange failed: {}", s)
            }
        }
    }
}

impl std::error::Error for OAuthListenerError {}

fn handle_client(
    mut stream: TcpStream,
    config: &OAuthListenerConfig,
) -> Result<OAuthTokens, OAuthListenerError> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| {
        OAuthListenerError::InvalidRequest(format!("Failed to clone stream: {}", e))
    })?);

    let mut request_line = String::new();
    reader.read_line(&mut request_line).map_err(|e| {
        OAuthListenerError::InvalidRequest(format!("Failed to read request line: {}", e))
    })?;

    log::debug!("OAuth callback received: {}", request_line.trim());

    // Parse GET /...?... HTTP/1.1
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 || parts[0] != "GET" {
        send_error_response(&mut stream, "Invalid request method");
        return Err(OAuthListenerError::InvalidRequest(
            "Expected GET request".into(),
        ));
    }

    let dummy_url = format!("http://localhost{}", parts[1]);
    let parsed_url = url::Url::parse(&dummy_url).map_err(|e| {
        send_error_response(&mut stream, "Invalid URL");
        OAuthListenerError::InvalidRequest(format!("Failed to parse URL: {}", e))
    })?;

    let params: std::collections::HashMap<String, String> = parsed_url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    if params.is_empty() {
        send_error_response(&mut stream, "No query parameters");
        return Err(OAuthListenerError::InvalidRequest(
            "No query parameters in request".into(),
        ));
    }

    if params.contains_key("access_token") {
        handle_default_mode_callback(&mut stream, &params)
    } else if params.contains_key("code") {
        handle_self_hosted_callback(&mut stream, &params, config)
    } else {
        send_error_response(&mut stream, "No valid OAuth parameters found");
        Err(OAuthListenerError::MissingParameter(
            "access_token or code".into(),
        ))
    }
}

// Handle callback from jump server (tokens already exchanged)
fn handle_default_mode_callback(
    stream: &mut TcpStream,
    params: &std::collections::HashMap<String, String>,
) -> Result<OAuthTokens, OAuthListenerError> {
    if let Some(status) = params.get("status") {
        if status != "ok" {
            let error_msg = params
                .get("error")
                .cloned()
                .unwrap_or_else(|| "Unknown error".into());
            send_error_response(stream, &error_msg);
            return Err(OAuthListenerError::AuthFailed(error_msg));
        }
    }

    let access_token = params
        .get("access_token")
        .ok_or_else(|| OAuthListenerError::MissingParameter("access_token".into()))?
        .clone();

    let refresh_token = params
        .get("refresh_token")
        .ok_or_else(|| OAuthListenerError::MissingParameter("refresh_token".into()))?
        .clone();

    send_success_response(stream);

    Ok(OAuthTokens {
        access_token,
        refresh_token,
    })
}

// Handle callback from osu! directly (need to exchange code for tokens)
fn handle_self_hosted_callback(
    stream: &mut TcpStream,
    params: &std::collections::HashMap<String, String>,
    config: &OAuthListenerConfig,
) -> Result<OAuthTokens, OAuthListenerError> {
    if let Some(error) = params.get("error") {
        let mut error_description = params
            .get("error_description")
            .cloned()
            .unwrap_or_else(|| error.clone());
        if let Some(error_hint) = params.get("hint") {
            error_description = format!("{error_description}.\nHint: {error_hint}");
        }
        send_error_response(stream, &error_description);
        return Err(OAuthListenerError::AuthFailed(error_description));
    }

    let code = params
        .get("code")
        .ok_or_else(|| {
            send_error_response(stream, "Missing authorization code");
            OAuthListenerError::MissingParameter("code".into())
        })?
        .clone();

    log::info!("Received authorization code, exchanging for tokens...");

    match exchange_code_for_tokens(&code, config) {
        Ok(tokens) => {
            send_success_response(stream);
            Ok(tokens)
        }
        Err(e) => {
            send_error_response(stream, &e.to_string());
            Err(e)
        }
    }
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    #[allow(dead_code)]
    token_type: String,
    #[allow(dead_code)]
    expires_in: u64,
}

#[derive(serde::Deserialize)]
struct TokenErrorResponse {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

fn exchange_code_for_tokens(
    code: &str,
    config: &OAuthListenerConfig,
) -> Result<OAuthTokens, OAuthListenerError> {
    let client = ureq::Agent::new_with_defaults();

    let form_data = [
        ("client_id".to_string(), config.client_id.clone()),
        ("client_secret".to_string(), config.client_secret.clone()),
        ("code".to_string(), code.to_string()),
        ("grant_type".to_string(), "authorization_code".to_string()),
        ("redirect_uri".to_string(), config.redirect_uri.clone()),
    ];

    log::debug!(
        "Exchanging code for tokens with client_id={}, redirect_uri={}",
        config.client_id,
        config.redirect_uri
    );

    let response = client
        .post("https://osu.ppy.sh/oauth/token")
        .send_form(form_data)
        .map_err(|e| {
            log::error!("Token exchange request failed: {}", e);
            OAuthListenerError::TokenExchangeFailed(format!("Request failed: {}", e))
        })?;

    let status = response.status();
    let body = response.into_body().read_to_string().map_err(|e| {
        OAuthListenerError::TokenExchangeFailed(format!("Failed to read response: {}", e))
    })?;

    if status != 200 {
        log::error!("Token exchange failed with status {}: {}", status, body);
        if let Ok(error_resp) = serde_json::from_str::<TokenErrorResponse>(&body) {
            let msg = error_resp.error_description.unwrap_or(error_resp.error);
            return Err(OAuthListenerError::TokenExchangeFailed(msg));
        }

        return Err(OAuthListenerError::TokenExchangeFailed(format!(
            "HTTP {}: {}",
            status, body
        )));
    }

    let token_resp: TokenResponse = serde_json::from_str(&body).map_err(|e| {
        log::error!("Failed to parse token response: {} - body: {}", e, body);
        OAuthListenerError::TokenExchangeFailed(format!("Invalid response: {}", e))
    })?;

    log::info!("Successfully exchanged code for tokens");

    Ok(OAuthTokens {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token,
    })
}

fn send_success_response(stream: &mut TcpStream) {
    let body = r#"<!DOCTYPE html>
<html>
<head><title>steel - OAuth Success</title></head>
<body style="font-family: sans-serif; text-align: center; padding-top: 50px;">
<h1>Authentication Successful</h1>
<p>You can close this window and return to steel.</p>
</body>
</html>"#;

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn send_error_response(stream: &mut TcpStream, error: &str) {
    let body = format!(
        r#"<!DOCTYPE html>
<html>
<head><title>steel - OAuth Error</title></head>
<body style="font-family: sans-serif; text-align: center; padding-top: 50px;">
<h1>Authentication Failed</h1>
<p>{}</p>
<p>Please try again.</p>
</body>
</html>"#,
        html_escape::encode_safe(error)
    );

    let response = format!(
        "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

pub fn start_oauth_listener<F>(
    port: u16,
    config: OAuthListenerConfig,
    app_queue: UnboundedSender<AppMessageIn>,
    on_complete: F,
) -> Result<(), OAuthListenerError>
where
    F: FnOnce() + Send + 'static,
{
    let addr = format!("127.0.0.1:{}", port);
    log::info!("Starting OAuth listener on {}", addr);
    let listener = TcpListener::bind(&addr).map_err(OAuthListenerError::BindFailed)?;

    listener
        .set_nonblocking(false)
        .map_err(OAuthListenerError::BindFailed)?;

    std::thread::spawn(move || {
        match listener.accept() {
            // If anything, this is a single-shot listener.
            Ok((stream, addr)) => {
                log::info!("OAuth callback connection from {}", addr);

                match handle_client(stream, &config) {
                    Ok(tokens) => {
                        log::info!("OAuth tokens received successfully");
                        if let Err(e) = app_queue.send(AppMessageIn::http_tokens_received(tokens)) {
                            log::error!("Failed to send tokens to app: {}", e);
                        }
                    }
                    Err(e) => {
                        log::error!("OAuth callback handling failed: {}", e);
                        if let Err(send_err) =
                            app_queue.send(AppMessageIn::http_token_error(e.to_string()))
                        {
                            log::error!("Failed to send error to app: {}", send_err);
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to accept OAuth callback connection: {}", e);
                if let Err(send_err) = app_queue.send(AppMessageIn::http_token_error(format!(
                    "Accept failed: {}",
                    e
                ))) {
                    log::error!("Failed to send error to app: {}", send_err);
                }
            }
        }

        on_complete();
        log::info!("OAuth listener stopped");
    });

    Ok(())
}

pub fn build_oauth_url(jump_server_url: &str, local_port: u16, scopes: &[&str]) -> String {
    let scopes_str = scopes.join("+");
    format!(
        "{}?local_port={}&scopes={}",
        jump_server_url, local_port, scopes_str
    )
}
