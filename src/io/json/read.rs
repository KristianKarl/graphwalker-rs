use std::fs;

pub fn read(input_file: &str) -> crate::graph::model::Models {
    dbg!(input_file);
    let json = fs::read_to_string(input_file).expect("Unable to read file");
    let models: crate::graph::model::Models = serde_json::from_str(&json).expect("Unable to parse");
    return models;
}
