use steel_core::settings::chat::OAuthMode;

const OSU_TOKEN_URL: &str = "https://osu.ppy.sh/oauth/token";

#[derive(Debug, Clone)]
pub struct TokenRefreshConfig {
    pub oauth_mode: OAuthMode,
    pub client_id: u64,
    pub client_secret: String,
    pub jump_server_url: String,
}

#[derive(Debug)]
pub enum TokenRefreshError {
    RequestFailed(String),
    Rejected(String),
    InvalidResponse(String),
}

impl std::fmt::Display for TokenRefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenRefreshError::RequestFailed(s) => write!(f, "Token refresh failed: {}", s),
            TokenRefreshError::Rejected(s) => write!(f, "Token refresh rejected: {}", s),
            TokenRefreshError::InvalidResponse(s) => {
                write!(f, "Invalid token refresh response: {}", s)
            }
        }
    }
}

impl std::error::Error for TokenRefreshError {}

#[derive(Debug, Clone)]
pub struct RefreshedTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

#[derive(serde::Deserialize)]
struct TokenErrorResponse {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

pub fn jump_server_refresh_url(jump_server_url: &str) -> String {
    format!("{}/refresh", jump_server_url.trim_end_matches('/'))
}

pub fn refresh_tokens(
    config: &TokenRefreshConfig,
    refresh_token: &str,
) -> Result<RefreshedTokens, TokenRefreshError> {
    let client: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();

    let response = match config.oauth_mode {
        OAuthMode::Default => {
            let url = jump_server_refresh_url(&config.jump_server_url);
            log::debug!("Refreshing tokens via jump server: {}", url);
            client
                .post(&url)
                .send_form([("refresh_token".to_string(), refresh_token.to_string())])
        }
        OAuthMode::SelfHosted => {
            log::debug!(
                "Refreshing tokens directly with client_id={}",
                config.client_id
            );
            client.post(OSU_TOKEN_URL).send_form([
                ("grant_type".to_string(), "refresh_token".to_string()),
                ("refresh_token".to_string(), refresh_token.to_string()),
                ("client_id".to_string(), config.client_id.to_string()),
                ("client_secret".to_string(), config.client_secret.clone()),
            ])
        }
    }
    .map_err(|e| {
        log::error!("Token refresh request failed: {}", e);
        TokenRefreshError::RequestFailed(format!("Request failed: {}", e))
    })?;

    let status = response.status();
    let body = response.into_body().read_to_string().map_err(|e| {
        TokenRefreshError::InvalidResponse(format!("Failed to read response: {}", e))
    })?;

    if status != 200 {
        log::error!("Token refresh failed with status {}: {}", status, body);

        let msg = match serde_json::from_str::<TokenErrorResponse>(&body) {
            Ok(error_resp) => error_resp.error_description.unwrap_or(error_resp.error),
            Err(_) => format!("HTTP {}: {}", status, body),
        };

        return Err(if (400..500).contains(&status.as_u16()) {
            TokenRefreshError::Rejected(msg)
        } else {
            TokenRefreshError::RequestFailed(msg)
        });
    }

    let token_resp: TokenResponse = serde_json::from_str(&body).map_err(|e| {
        log::error!(
            "Failed to parse token refresh response: {} - body: {}",
            e,
            body
        );
        TokenRefreshError::InvalidResponse(format!("Invalid response: {}", e))
    })?;

    log::info!("Successfully refreshed tokens");

    Ok(RefreshedTokens {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token,
        expires_in: token_resp.expires_in,
    })
}
