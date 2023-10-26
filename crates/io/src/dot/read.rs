use std::collections::BTreeMap;

use graph::Models;
use log::{debug, error};

#[must_use]
pub fn read(input_file: &str) -> Models {
    debug!("{}", input_file);
    error!("Feature not implemented");

    Models {
        models: BTreeMap::default(),
        start_element_id: None,
    }
}
