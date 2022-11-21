use log::debug;
use graph::Models;

pub fn read(input_file: &str) -> Models {
    debug!("{}", input_file);
    let models = Models { models: vec![] };
    return models;
}
