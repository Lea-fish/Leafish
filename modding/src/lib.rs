use std::any::Any;
use libloading::{Library, Symbol, Error};
use std::sync::Arc;
use dashmap::DashMap;
use std::path::Path;
use log::warn;
extern crate serde;
use serde::{Deserialize, Serialize};

pub trait Plugin: Any + Send + Sync {

    fn on_enable(&self);

    fn on_disable(&self) {}

    fn get_name(&self) -> &'static str;

}

pub struct PluginManager {

    plugins: Arc<DashMap<String, WrappedPlugin>>,

}

impl PluginManager {

    pub fn load_plugin(&self, path: String) -> bool {
        let raw_plugin_meta = std::fs::read_to_string(format!("{}/meta.json", path));
        let plugin_meta = if let Ok(meta) = raw_plugin_meta {
            if let Ok(meta) = serde_json::from_str(&*meta) {
                meta
            } else {
                PluginMeta::default()
            }
        } else {
            PluginMeta::default()
        };

        let potential_library = unsafe { Library::new(format!("{}/bin", path)) };
        if let Ok(library) = potential_library {
            let create_plugin: Result<Symbol<fn() -> Box<dyn Plugin>>, Error> = unsafe { library.get(b"create_plugin") };
            if let Ok(create_plugin) = create_plugin {
                let wrapped_plugin = WrappedPlugin {
                    plugin: create_plugin(),
                    _library: library,
                    meta: plugin_meta,
                };
                let file_name = Path::new(&path).file_name().unwrap();
                self.plugins.clone().insert(String::from(file_name.to_str().unwrap()), wrapped_plugin);
                return true;
            } else {
                let file_name = Path::new(&path).file_name().unwrap();
                warn!("There is no \"create_plugin\" function available in {}!", file_name.to_str().unwrap());
            }
        }
        false
    }

    pub fn unload_plugin(&self, plugin: String) -> bool {
        if let Some(plugin) = self.plugins.clone().remove(&*plugin) {
            plugin.1.plugin.on_disable();
            return true;
        }
        false
    }

    pub fn unload_all(&self) {
        for plugin in self.plugins.clone().iter() {
            plugin.plugin.on_disable();
        }
        self.plugins.clear();
    }

}

pub struct WrappedPlugin {

    plugin: Box<dyn Plugin>,
    _library: Library,
    meta: PluginMeta,

}

impl WrappedPlugin {

    pub fn name(&self) -> &'static str {
        self.plugin.get_name()
    }

    pub fn version(&self) -> usize {
        self.meta.version
    }

    pub fn authors(&self) -> &Vec<String> {
        &self.meta.authors
    }

    pub fn permissions(&self) -> &Vec<PluginPermission> {
        &self.meta.permissions
    }

}

#[derive(Serialize, Deserialize, Default)]
pub struct PluginMeta {

    version: usize,
    authors: Vec<String>,
    permissions: Vec<PluginPermission>, // TODO: Implement this!
    modified: bool,

}


#[derive(Serialize, Deserialize)]
pub enum PluginPermission {

    File,
    Network,
    Device, // grants permission for devices and drivers
    All,

}