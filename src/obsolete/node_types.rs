// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::NodeTypes;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: NodeTypes = serde_json::from_str(&json).unwrap();
// }

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

pub type NodeTypes = Vec<NodeType>;

#[derive(Serialize, Deserialize)]
pub struct NodeType {
    #[serde(rename = "type")]
    node_type_type: String,

    named: bool,

    subtypes: Option<Vec<Type>>,

    fields: Option<HashMap<String, Children>>,

    children: Option<Children>,
}

#[derive(Serialize, Deserialize)]
pub struct Children {
    multiple: bool,

    required: bool,

    types: Vec<Type>,
}

#[derive(Serialize, Deserialize)]
pub struct Type {
    #[serde(rename = "type")]
    type_type: String,

    named: bool,
}
