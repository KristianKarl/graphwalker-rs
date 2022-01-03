use std::path::Path;
use std::ffi::OsStr;


fn get_extension_from_filename(file_name: &str) -> Option<&str> {
    Path::new(file_name)
        .extension()
        .and_then(OsStr::to_str)
}

pub fn read(input_file: &str) -> crate::graph::model::Models {
    debug!("{}", input_file);

    if std::path::Path::new(input_file).exists() {
        let suffix = get_extension_from_filename(input_file);
        
        match suffix {
            Some("json") => {
                return crate::io::json::read::read(input_file);
            },
            Some("dot") => {
                return crate::io::dot::read::read(input_file);
            },
            _ => panic!("{}", format!("Unknown suffix. Not implemented for file: {}", input_file)),
        }
    } else {
        panic!("Could not open and read file: {}", input_file);
    }
}