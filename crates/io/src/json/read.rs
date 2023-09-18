use graph::Models;
use log::debug;
use std::fs;

#[must_use]
pub fn read(input_file: &str) -> Result<Models, String> {
    debug!("{}", input_file);
    let res = fs::read_to_string(input_file);
    match res {
        Ok(json_str) => {
            let models: Models = serde_json::from_str(&json_str)
                .expect(&format!("Unable to parse file: {}", input_file));
            Ok(models)
        }
        Err(why) => {
            log::error!("{:?}", why);
            Err(why.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    fn resource_path(resource: &str) -> std::path::PathBuf {
        let mut path = std::path::PathBuf::new();
        path.push(env!("CARGO_MANIFEST_DIR"));
        path.push("..");
        path.push("..");
        path.push("resources");
        path.push("models");
        path.push(resource);
        path
    }

    #[test]
    fn read_valid_jason_file() {
        let json = fs::read_to_string(resource_path(resource_path("login.json").to_str().unwrap()))
            .expect("Unable to read file");
        let models: Models = serde_json::from_str(&json).expect("Unable to parse");

        assert_eq!(models.models.len(), 1);

        let m = models
            .models
            .get("853429e2-0528-48b9-97b3-7725eafbb8b5")
            .expect("Expected a Model");
        assert_eq!(m.vertices.len(), 3);
        assert_eq!(m.edges.len(), 9);

        let v = m.vertices.get("n2").expect("Expected a Vertex");
        assert_eq!(
            v.name.clone().expect("Expected the name of the vertex"),
            "v_LoginPrompted"
        );
    }
}
