use graph::Models;
use log::debug;
use std::fs;

pub fn read(input_file: &str) -> Models {
    debug!("{}", input_file);
    let json = fs::read_to_string(input_file).expect("Unable to read file");
    let models: Models = serde_json::from_str(&json).expect("Unable to parse");
    return models;
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn read_valid_jason_file() {
        let json = fs::read_to_string("tests/models/login.json").expect("Unable to read file");
        let models: Models = serde_json::from_str(&json).expect("Unable to parse");

        assert_eq!(models.models.len(), 1);

        let m = models.models.get("853429e2-0528-48b9-97b3-7725eafbb8b5").unwrap();
        assert_eq!(m.vertices.len(), 4);
        assert_eq!(m.edges.len(), 9);

        let v = m.vertices.get("n2").unwrap();
        assert_eq!(v.name.clone().unwrap(), "v_LoginPrompted");
    }
   
}