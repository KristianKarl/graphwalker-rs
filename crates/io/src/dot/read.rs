use std::collections::HashMap;

use graph::Models;
use log::{debug, error};

#[must_use] pub fn read(input_file: &str) -> Models {
    debug!("{}", input_file);
    error!("Feature not implemented");
    
    Models {
        models: HashMap::new(),
    }
}
