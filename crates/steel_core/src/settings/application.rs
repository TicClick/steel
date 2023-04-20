use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Application {
    pub plugins: Plugins,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Plugins {
    pub enabled: bool,
}
