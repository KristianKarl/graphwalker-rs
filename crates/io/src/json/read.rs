use graph::Models;
use log::debug;
use std::{fs, sync::Arc};

pub fn read(input_file: &str) -> Result<Arc<Models>, String> {
    debug!("{}", input_file);
    let res = fs::read_to_string(input_file);
    match res {
        Ok(json_str) => {
            match serde_json::from_str(&json_str) {
                Ok(models) => Ok(models),
                Err(err) => {
                    let msg = format!("Unable to parse file: {}. Failed with reason: {}", input_file, err);
                    log::error!("{}", msg);
                    Err(msg)
                }
             }
        }
        Err(why) => {
            log::error!("{:?}", why);
            Err(why.to_string())
        }
    }
}