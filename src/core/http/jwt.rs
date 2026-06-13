#[derive(Debug, Clone, Copy)]
pub struct AccessTokenTiming {
    pub issued_at: f64,
    pub not_before: f64,
    pub expires_at: f64,
}

impl AccessTokenTiming {
    pub fn age_secs(&self) -> f64 {
        chrono::Utc::now().timestamp() as f64 - self.issued_at
    }

    pub fn delay_until_valid_secs(&self) -> f64 {
        self.not_before - chrono::Utc::now().timestamp() as f64
    }
}

#[derive(serde::Deserialize)]
struct JwtClaims {
    iat: f64,
    nbf: f64,
    exp: f64,
}

pub fn access_token_timing(token: &str) -> Option<AccessTokenTiming> {
    let payload_base64 = token.split('.').nth(1)?;

    let payload_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        payload_base64,
    )
    .map_err(|e| log::warn!("Failed to decode JWT payload: {e}"))
    .ok()?;

    let payload_str = std::str::from_utf8(&payload_bytes)
        .map_err(|e| log::warn!("Failed to parse JWT payload as UTF-8: {e}"))
        .ok()?;

    let claims: JwtClaims = serde_json::from_str(payload_str)
        .map_err(|e| log::warn!("Failed to parse JWT claims: {e}"))
        .ok()?;

    Some(AccessTokenTiming {
        issued_at: claims.iat,
        not_before: claims.nbf,
        expires_at: claims.exp,
    })
}
