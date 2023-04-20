use std::fmt::Display;
/// Dynamic library loading tools, as seen at https://michael-f-bryan.github.io/rust-ffi-guide/dynamic_loading.html

use std::{any::Any, error::Error};
use std::ffi::OsStr;

use eframe::egui;
use libloading::{Library, Symbol};
use steel_core::chat::Message;
use steel_core::ipc::client::CoreClient;

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

pub trait Plugin: Any + Send + Sync {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;

    fn on_plugin_load(&self) {}
    fn on_plugin_unload(&self) {}

    fn show_user_context_menu(&self, _ui: &mut egui::Ui, _core: &CoreClient, _chat_name: &str, _message: &Message) {}
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

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    loaded_libraries: Vec<Library>,
}

impl PluginManager {
    pub fn new() -> PluginManager {
        PluginManager {
            plugins: Vec::new(),
            loaded_libraries: Vec::new(),
        }
    }

    pub unsafe fn load_plugin<P: AsRef<OsStr>>(&mut self, filename: P) -> Result<(), libloading::Error> {
        type PluginCreate = unsafe extern fn() -> *mut dyn Plugin;

        let lib = Library::new(filename.as_ref())?;
        self.loaded_libraries.push(lib);

        let lib = self.loaded_libraries.last().unwrap();
        let constructor: Symbol<PluginCreate> = lib.get(b"_plugin_create")?;
        let boxed_raw = constructor();

        let plugin = Box::from_raw(boxed_raw);
        log::debug!("Loaded plugin: {:?} {}", plugin.name(), plugin.version());
        plugin.on_plugin_load();
        self.plugins.push(plugin);

        Ok(())
    }

    pub fn has_plugins(&self) -> bool {
        !self.plugins.is_empty()
    }

    pub fn show_user_context_menu(&self, ui: &mut egui::Ui, core: &CoreClient, chat_name: &str, message: &Message) {
        log::debug!("Firing show_user_context_menu hooks");
        for plugin in &self.plugins {
            log::trace!("Firing show_user_context_menu for {:?}", plugin.name());
            plugin.show_user_context_menu(ui, core, chat_name, message)
        }
    }

    pub fn unload(&mut self) {
        log::debug!("Unloading plugins");
        for plugin in self.plugins.drain(..) {
            log::trace!("Firing on_plugin_unload for {:?}", plugin.name());
            plugin.on_plugin_unload();
        }
        for lib in self.loaded_libraries.drain(..) {
            drop(lib);
        }
    }

    pub fn installed(&self) -> Vec<(&str, &str)> {
        let mut ret = Vec::new();
        for p in &self.plugins {
            ret.push((p.name(), p.version()));
        }
        ret
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        if !self.plugins.is_empty() || !self.loaded_libraries.is_empty() {
            self.unload();
        }
    }
}