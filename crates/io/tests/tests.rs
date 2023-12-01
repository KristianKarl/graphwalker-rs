use std::fs;

use graph::Models;
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

#[derive(Default, Debug)]
struct TestData {
    file_name: String,
    num_of_models: usize,
    model_id: String,
    num_of_edges: usize,
    num_of_vertices: usize,
}

#[test]
fn test_read_valid_models() {
    let test_data = vec![
        TestData {
            file_name: "login.json".to_string(),
            num_of_models: 1,
            model_id: "login".to_string(),
            num_of_edges: 8,
            num_of_vertices: 3,
        },
        TestData {
            file_name: "petclinic.json".to_string(),
            num_of_models: 5,
            model_id: "3f6b365f-7011-4db6-b0cc-e19aa453d9b8".to_string(),
            num_of_edges: 7,
            num_of_vertices: 3,
        },
        TestData {
            file_name: "simple.json".to_string(),
            num_of_models: 1,
            model_id: "m1".to_string(),
            num_of_edges: 3,
            num_of_vertices: 3,
        },
    ];

    for data in test_data {
        let json = fs::read_to_string(resource_path(resource_path(&data.file_name).to_str().unwrap()))
            .unwrap();
        let models: Models = serde_json::from_str(&json).unwrap();

        assert_eq!(models.models.len(), data.num_of_models);

        let m = models.models.get(&data.model_id).unwrap();
        assert_eq!(m.vertices.read().unwrap().len(), data.num_of_vertices);
        assert_eq!(m.edges.read().unwrap().len(), data.num_of_edges);
    }
}
