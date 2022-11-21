use std::fs;
use log::debug;
use graph::Models;

pub fn read(input_file: &str) -> Models {
    debug!("{}", input_file);
    let json = fs::read_to_string(input_file).expect("Unable to read file");
    let models: Models = serde_json::from_str(&json).expect("Unable to parse");
    return models;
}