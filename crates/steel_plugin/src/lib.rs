// Dynamic library loading tools, as seen at https://michael-f-bryan.github.io/rust-ffi-guide/dynamic_loading.html

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::{any::Any, error::Error};

use eframe::egui;
use libloading::{Library, Symbol};
use steel_core::chat::Message;
use steel_core::ipc::client::CoreClient;
use steel_core::VersionString;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
pub enum PluginError {
    ValidationError(String),
}

impl Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for PluginError {}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum TextStyle {
    Bold,
    Italics,
    Underline,
    Strikethrough,
    Monospace,

    Highlight,
}

pub trait Plugin: Any + Send + Sync {
    fn name(&self) -> String;
    fn version(&self) -> String;
    fn plugin_system_version(&self) -> String;

    fn on_plugin_load(&self) {}
    fn show_user_context_menu(
        &self,
        _ui: &mut egui::Ui,
        _core: &CoreClient,
        _chat_name: &str,
        _message: &Message,
    ) {
    }
    fn handle_incoming_message(
        &mut self,
        _core: &CoreClient,
        _chat_name: &str,
        _message: &Message,
    ) {
    }
    fn handle_outgoing_message(
        &mut self,
        _core: &CoreClient,
        _chat_name: &str,
        _message: &Message,
    ) {
    }
    fn style_message(&self, _chat_name: &str, _message: &Message) -> Option<TextStyle> {
        None
    }
    fn style_username(&self, _chat_name: &str, _message: &Message) -> Option<TextStyle> {
        None
    }
    fn validate_message_input(&self, _core: &CoreClient, _chat_name: &str, _message: &Message) {}
    fn show_user_tooltip(&self, _chat_name: &str, _message: &Message) -> Option<String> {
        None
    }
}

impl Debug for dyn Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SomePlugin")
    }
}

#[macro_export]
macro_rules! declare_plugin {
    ($plugin_type:ty, $constructor:path) => {
        #[no_mangle]
        pub extern "C" fn _plugin_create() -> *mut dyn crate::Plugin {
            let constructor: fn() -> $plugin_type = $constructor;
            let object = constructor();
            let boxed: Box<dyn crate::Plugin> = Box::new(object);
            Box::into_raw(boxed)
        }
    };
}

fn is_dynamic_library(p: &std::fs::DirEntry) -> bool {
    let path = p.path();
    path.is_file() && {
        let ext = path.extension().unwrap_or_default();
        ext == "dll" || ext == "so"
    }
}

#[derive(Debug, Default)]
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    loaded_libraries: Vec<Library>,
    pub initialized: bool,
}

impl PluginManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn discover_plugins(&mut self, dir: &PathBuf) {
        match std::fs::read_dir(dir) {
            Err(e) => log::error!("failed to scan {:?} for plugins: {:?}", dir, e),
            Ok(it) => {
                for library in it.into_iter().filter_map(|elem| match elem {
                    Err(_) => None,
                    Ok(p) => match is_dynamic_library(&p) {
                        true => Some(p),
                        false => None,
                    },
                }) {
                    unsafe {
                        if let Err(e) = self.load_plugin(library.path()) {
                            log::error!("failed to load plugin {:?}: {:?}", library, e);
                        }
                    }
                }
            }
        }
        self.initialized = true;
    }

    pub unsafe fn load_plugin<P: std::fmt::Debug + AsRef<OsStr>>(
        &mut self,
        filename: P,
    ) -> Result<(), libloading::Error> {
        type PluginCreate = unsafe extern "C" fn() -> *mut dyn Plugin;

        let lib = Library::new(filename.as_ref())?;
        self.loaded_libraries.push(lib);

        let lib = self.loaded_libraries.last().unwrap();
        let constructor: Symbol<PluginCreate> = lib.get(b"_plugin_create")?;
        let boxed_raw = constructor();

        let plugin = Box::from_raw(boxed_raw);

        if plugin.plugin_system_version().semver().0 != VERSION.semver().0 {
            self.loaded_libraries.pop();
            log::error!(
                "Failed to load {:?} as a plugin due to mismatch of major version of plugin systems: {} (us) vs {} (plugin)",
                filename, VERSION, plugin.plugin_system_version()
            );
            return Err(libloading::Error::IncompatibleSize);
        }

        log::debug!("Loaded plugin: {:?} {}", plugin.name(), plugin.version());
        plugin.on_plugin_load();
        self.plugins.push(plugin);

        Ok(())
    }

    pub fn has_plugins(&self) -> bool {
        !self.plugins.is_empty()
    }

    pub fn show_user_context_menu(
        &self,
        ui: &mut egui::Ui,
        core: &CoreClient,
        chat_name: &str,
        message: &Message,
    ) {
        if !self.has_plugins() {
            return;
        }

        log::debug!("Firing show_user_context_menu hooks");
        for plugin in &self.plugins {
            log::trace!("Firing show_user_context_menu for {:?}", plugin.name());
            plugin.show_user_context_menu(ui, core, chat_name, message)
        }
    }

    pub fn show_user_tooltip(&self, chat_name: &str, message: &Message) -> Option<String> {
        if !self.has_plugins() {
            return None;
        }

        let mut tooltips = Vec::new();
        for plugin in &self.plugins {
            log::trace!(
                "Collecting chat_name tooltip for {} from {:?}",
                message.username,
                plugin.name()
            );
            if let Some(tt) = plugin.show_user_tooltip(chat_name, message) {
                tooltips.push(tt);
            }
        }
        match tooltips.is_empty() {
            true => None,
            false => Some(tooltips.join("\n")),
        }
    }

    pub fn handle_incoming_message(
        &mut self,
        core: &CoreClient,
        chat_name: &str,
        message: &Message,
    ) {
        if !self.has_plugins() {
            return;
        }
        for plugin in &mut self.plugins {
            log::trace!("Firing handle_incoming_message for {:?}", plugin.name());
            plugin.handle_incoming_message(core, chat_name, message);
        }
    }

    pub fn handle_outgoing_message(
        &mut self,
        core: &CoreClient,
        chat_name: &str,
        message: &Message,
    ) {
        if !self.has_plugins() {
            return;
        }
        for plugin in &mut self.plugins {
            log::trace!("Firing handle_outgoing_message for {:?}", plugin.name());
            plugin.handle_outgoing_message(core, chat_name, message);
        }
    }

    pub fn style_message(&self, chat_name: &str, message: &Message) -> Option<BTreeSet<TextStyle>> {
        if !self.has_plugins() {
            return None;
        }
        let mut ret = BTreeSet::new();
        for plugin in &self.plugins {
            log::trace!("Firing style_message for {:?}", plugin.name());
            if let Some(d) = plugin.style_message(chat_name, message) {
                ret.insert(d);
            }
        }
        Some(ret)
    }

    pub fn style_username(
        &self,
        chat_name: &str,
        message: &Message,
    ) -> Option<BTreeSet<TextStyle>> {
        if !self.has_plugins() {
            return None;
        }
        let mut ret = BTreeSet::new();
        for plugin in &self.plugins {
            log::trace!("Firing style_username for {:?}", plugin.name());
            if let Some(d) = plugin.style_username(chat_name, message) {
                ret.insert(d);
            }
        }
        Some(ret)
    }

    pub fn validate_message_input(&self, core: &CoreClient, chat_name: &str, message: &Message) {
        if !self.has_plugins() {
            return;
        }
        for plugin in &self.plugins {
            log::trace!("Firing validate_message_input for {:?}", plugin.name());
            plugin.validate_message_input(core, chat_name, message);
        }
    }

    pub fn installed(&self) -> Vec<(String, String)> {
        let mut ret = Vec::new();
        for p in &self.plugins {
            ret.push((p.name(), p.version()));
        }
        ret
    }
}
