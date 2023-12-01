use std::{collections::BTreeMap, sync::Arc};

use graph::Models;
use log::{debug, error};

#[must_use]
pub fn read(input_file: &str) -> Arc<Models> {
    debug!("{}", input_file);
    error!("Feature not implemented");

    Arc::new(Models {
        models: Arc::new(BTreeMap::default()),
        start_element_id: None,
    })
}
