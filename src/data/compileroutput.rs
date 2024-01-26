// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::CompilerOutput;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: CompilerOutput = serde_json::from_str(&json).unwrap();
// }
// Generated using [quicktype](
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerOutputElement {
    pub(crate) reason: String,
    pub(crate) package_id: Option<String>,
    pub(crate) manifest_path: Option<String>,
    pub(crate) target: Option<Target>,
    pub(crate) profile: Option<Profile>,
    pub(crate) features: Option<Vec<String>>,
    pub(crate) filenames: Option<Vec<String>>,
    pub(crate) executable: Option<String>,
    pub(crate) fresh: Option<bool>,
    pub(crate) linked_libs: Option<Vec<Option<serde_json::Value>>>,
    pub(crate) linked_paths: Option<Vec<Option<serde_json::Value>>>,
    pub(crate) cfgs: Option<Vec<String>>,
    pub(crate) env: Option<Vec<Option<serde_json::Value>>>,
    pub(crate) out_dir: Option<String>,
    pub(crate) success: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub(crate) opt_level: String,
    pub(crate) debuginfo: Option<i64>,
    pub(crate) debug_assertions: bool,
    pub(crate) overflow_checks: bool,
    pub(crate) test: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub(crate) kind: Vec<String>,
    pub(crate) crate_types: Vec<String>,
    pub(crate) name: String,
    pub(crate) src_path: String,
    pub(crate) edition: String,
    pub(crate) doc: bool,
    pub(crate) doctest: bool,
    pub(crate) test: bool,
    #[serde(rename = "required-features")]
    pub(crate) required_features: Option<Vec<String>>,
}
