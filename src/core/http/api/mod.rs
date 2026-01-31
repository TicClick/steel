use rosu_v2::prelude::Scopes;

pub mod client;
pub use client::Client;

fn default_scopes() -> Vec<(Scopes, &'static str)> {
    vec![
        (Scopes::Public, "public"),
        (Scopes::Identify, "identify"),
        (Scopes::ChatRead, "chat.read"),
        (Scopes::ChatWrite, "chat.write"),
        (Scopes::ChatWriteManage, "chat.write_manage"),
    ]
}

pub fn osu_api_default_scopes_str() -> Vec<&'static str> {
    default_scopes().into_iter().map(|f| f.1).collect()
}

pub fn osu_api_default_scopes() -> Scopes {
    default_scopes()
        .into_iter()
        .map(|f| f.0)
        .reduce(|acc, e| acc | e)
        .expect("Failed to construct default OAuth scopes")
}
