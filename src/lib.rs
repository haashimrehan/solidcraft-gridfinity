//! Gridfinity generator for SolidCraft.
//!
//! Two entries: `make-bin` and `make-baseplate`. Each one runs ordinary SolidCraft commands
//! (sketches, extrudes, a pattern), so what lands in the design is an editable timeline, not a
//! mesh. The plugin needs no files, no network and no clock.

pub mod grid;

use grid::{Baseplate, Bin};
use serde_json::Value;

/// What a build needs from the host: run one command with JSON args, get the JSON reply.
pub trait Runner {
    fn run(&mut self, command: &str, args: Value) -> Result<Value, String>;
}

pub use grid::{make_baseplate, make_bin};

/// Parse and check the parameters of an entry without building anything.
pub fn check(entry: &str, params: &Value) -> Result<(), grid::Invalid> {
    match entry {
        "make-bin" => Bin::from_json(params).validate(),
        "make-baseplate" => Baseplate::from_json(params).validate(),
        other => Err(("entry".into(), format!("unknown entry {other}"))),
    }
}

#[cfg(target_arch = "wasm32")]
mod guest {
    wit_bindgen::generate!({ path: "wit", world: "command-plugin" });

    use super::*;
    use craft::plugin::{
        commands, log,
        types::{FieldFailure, PluginError},
    };
    use serde_json::json;

    /// Runs commands on the host and keeps the first failure as the host reported it.
    struct Host {
        failure: Option<PluginError>,
    }

    impl Runner for Host {
        fn run(&mut self, command: &str, args: Value) -> Result<Value, String> {
            match commands::run(command, &args.to_string()) {
                Ok(s) => Ok(serde_json::from_str(&s).unwrap_or(Value::Null)),
                Err(e) => {
                    let text = format!("{command}: {e:?}");
                    self.failure.get_or_insert(e);
                    Err(text)
                }
            }
        }
    }

    struct Gridfinity;

    impl exports::craft::plugin::entry::Guest for Gridfinity {
        fn execute(entry: String, params: String) -> Result<String, PluginError> {
            let v: Value = serde_json::from_str(&params).unwrap_or(json!({}));
            if let Err((field, reason)) = check(&entry, &v) {
                return Err(PluginError::InvalidInput(FieldFailure { field, reason }));
            }
            let mut host = Host { failure: None };
            let built = match entry.as_str() {
                "make-bin" => {
                    let b = Bin::from_json(&v);
                    log::log(log::Level::Info, &format!("making a {}x{}x{} bin", b.width, b.depth, b.height));
                    make_bin(&mut host, &b)
                }
                _ => {
                    let p = Baseplate::from_json(&v);
                    log::log(log::Level::Info, &format!("making a {}x{} baseplate", p.width, p.depth));
                    make_baseplate(&mut host, &p)
                }
            };
            match built {
                Ok(v) => Ok(v.to_string()),
                Err(text) => Err(host.failure.take().unwrap_or(PluginError::Trap(text))),
            }
        }
    }

    export!(Gridfinity);
}
