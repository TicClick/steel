use std::sync::{Arc, Mutex};

use steel_core::ipc::server::AppMessageIn;
use steel_core::settings::chat::OAuthMode;
use tokio::sync::mpsc::UnboundedSender;

use super::api::osu_api_default_scopes_str;
use super::oauth_listener::{
    build_oauth_url, start_oauth_listener, OAuthListenerConfig, OAuthListenerError,
};

pub struct OAuthFlowParams {
    pub local_port: u16,
    pub oauth_mode: OAuthMode,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub jump_server_url: String,
}

#[derive(Debug, Clone, PartialEq)]
struct ActiveListener {
    port: u16,
}

#[derive(Debug, Default)]
struct OAuthFlowState {
    active_listener: Option<ActiveListener>,
}

#[derive(Debug, Clone)]
pub struct OAuthFlowManager {
    state: Arc<Mutex<OAuthFlowState>>,
}

impl Default for OAuthFlowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthFlowManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(OAuthFlowState::default())),
        }
    }

    pub fn start_oauth_flow(
        &self,
        params: OAuthFlowParams,
        app_queue: UnboundedSender<AppMessageIn>,
    ) -> Result<String, OAuthListenerError> {
        {
            let state = self.state.lock().unwrap();
            if let Some(ref active) = state.active_listener {
                if active.port == params.local_port {
                    log::info!(
                        "OAuth listener already running on port {}, reusing (config changes will apply on next login)",
                        params.local_port
                    );
                    return Ok(self.build_auth_url(&params));
                }
            }
        }

        let new_listener = ActiveListener {
            port: params.local_port,
        };

        let config = OAuthListenerConfig {
            mode: params.oauth_mode.clone(),
            client_id: params.client_id.clone(),
            client_secret: params.client_secret.clone(),
            redirect_uri: params.redirect_uri.clone(),
        };

        let state_clone = Arc::clone(&self.state);
        let listener_info = new_listener.clone();

        start_oauth_listener(params.local_port, config, app_queue, move || {
            let mut state = state_clone.lock().unwrap();
            if state.active_listener.as_ref() == Some(&listener_info) {
                state.active_listener = None;
                log::debug!(
                    "OAuth listener on port {} marked as stopped",
                    listener_info.port
                );
            }
        })?;

        {
            let mut state = self.state.lock().unwrap();
            state.active_listener = Some(new_listener);
        }

        Ok(self.build_auth_url(&params))
    }

    fn build_auth_url(&self, params: &OAuthFlowParams) -> String {
        let oauth_scopes = osu_api_default_scopes_str();
        match params.oauth_mode {
            OAuthMode::Default => {
                build_oauth_url(&params.jump_server_url, params.local_port, &oauth_scopes)
            }
            OAuthMode::SelfHosted => {
                let mut url = url::Url::parse("https://osu.ppy.sh/oauth/authorize").unwrap();
                url.query_pairs_mut()
                    .append_pair("client_id", &params.client_id)
                    .append_pair("redirect_uri", &params.redirect_uri)
                    .append_pair("response_type", "code")
                    .append_pair("scope", &oauth_scopes.join(" "));
                url.to_string()
            }
        }
    }

    pub fn is_listener_active(&self) -> bool {
        self.state.lock().unwrap().active_listener.is_some()
    }
}
