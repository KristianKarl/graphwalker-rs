use graph::Models;
use log::{debug, trace};
use std::ffi::OsStr;
use std::path::Path;

pub mod dot;
pub mod json;

fn get_extension_from_filename(file_name: &str) -> Option<&str> {
    Path::new(file_name).extension().and_then(OsStr::to_str)
}

pub fn read(input_file: &str) -> Result<Models, &str> {
    debug!("{}", input_file);

    if std::path::Path::new(input_file).exists() {
        let suffix = get_extension_from_filename(input_file);
        trace!("Suffix: {}", suffix.unwrap());

        match suffix {
            Some("json") => {
                return Ok(json::read::read(input_file));
            }
            Some("dot") => {
                return Ok(dot::read::read(input_file));
            }
            _ => {
                debug!("Suffix for file is not yet implemented: {}", input_file);
                return Err("File type is not implemented");
            }
        }
    } else {
        return Err("Could not open file");
    }
}
