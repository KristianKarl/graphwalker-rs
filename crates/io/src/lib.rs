use graph::Models;
use log::{debug, trace};
use std::ffi::OsStr;
use std::path::Path;
use std::sync::Arc;

#[path = "dot/read.rs"]
pub mod dot_read;
#[path = "dot/write.rs"]
pub mod dot_write;
#[path = "json/read.rs"]
pub mod json_read;
#[path = "json/write.rs"]
pub mod json_write;
#[path = "parsers/generator.rs"]
pub mod parsers_generator;

fn get_extension_from_filename(file_name: &str) -> Option<&str> {
    Path::new(file_name).extension().and_then(OsStr::to_str)
}

pub fn read(input_file: &str) -> Result<Arc<Models>, String> {
    debug!("{}", input_file);

    if std::path::Path::new(input_file).exists() {
        let suffix = get_extension_from_filename(input_file);
        trace!("Suffix: {:?}", suffix);

        match suffix {
            Some("json") => match json_read::read(input_file) {
                Ok(models) => Ok(models),
                Err(why) => Err(why),
            },
            Some("dot") => Ok(dot_read::read(input_file)),
            _ => {
                debug!("Suffix for file is not yet implemented: {}", input_file);
                Err("File type is not implemented".to_string())
            }
        }
    } else {
        Err("Could not open file".to_string())
    }
}
