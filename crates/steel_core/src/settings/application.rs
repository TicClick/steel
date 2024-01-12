use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Application {
    #[serde(default)]
    pub autoupdate: AutoUpdate,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AutoUpdate {
    pub enabled: bool,
}
