use assert_json_diff::assert_json_eq;
use machine::{Machine, MachineStatus};
use pretty_assertions::assert_eq;
use serde_json::json;

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
fn walk_multiple_model() {
    let mut machine = Machine::new();
    machine.seed(946892979);
    let res = machine.load_models(
        io::json_read::read(resource_path("simpleMultiModel.json").to_str().unwrap())
            .expect("Expexted the test file to be loaded"),
    );
    assert_eq!(res.is_ok(), true);

    let res = machine.walk();
    assert_eq!(
        res.is_ok(),
        true,
        "{:?}",
        Err::<(), Result<(), String>>(res)
    );

    let expected = json!([
        "{\"model_id\":\"m2\",\"element_id\":\"v1\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"e2\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v2\"}",
        "{\"model_id\":\"m1\",\"element_id\":\"v3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v2\"}",
        "{\"model_id\":\"m1\",\"element_id\":\"v3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v2\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"e3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"e1\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v1\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"e2\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v2\"}",
        "{\"model_id\":\"m1\",\"element_id\":\"v3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v2\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"e3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"e1\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v1\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"e2\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v2\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"e3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"e1\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v1\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"e2\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v2\"}",
        "{\"model_id\":\"m1\",\"element_id\":\"v3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v2\"}",
        "{\"model_id\":\"m1\",\"element_id\":\"v3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v2\"}",
        "{\"model_id\":\"m1\",\"element_id\":\"v3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v2\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"e3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v3\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"e1\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v1\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"e2\"}",
        "{\"model_id\":\"m2\",\"element_id\":\"v2\"}",
        "{\"model_id\":\"m1\",\"element_id\":\"v3\"}",
        "{\"model_id\":\"m1\",\"element_id\":\"e1\"}",
        "{\"model_id\":\"m1\",\"element_id\":\"v1\"}",
        "{\"model_id\":\"m1\",\"element_id\":\"e2\"}",
        "{\"model_id\":\"m1\",\"element_id\":\"v2\"}",
        "{\"model_id\":\"m1\",\"element_id\":\"e3\"}",
    ]);

    let actual: Vec<String> = machine
        .profile
        .steps
        .iter()
        .map(|p| serde_json::to_string(&p).unwrap())
        .collect();

    assert_json_eq!(expected, actual);
}

#[test]
fn walk_single_model() {
    let mut machine = Machine::new();
    let res = machine.load_models(
        io::json_read::read(resource_path("simpleSingleModel.json").to_str().unwrap())
            .expect("Expexted the test file to be loaded"),
    );
    assert_eq!(res.is_ok(), true);

    let res = machine.walk();
    assert_eq!(
        res.is_ok(),
        true,
        "{:?}",
        Err::<(), Result<(), String>>(res)
    );

    let expected = vec!["v1", "e2", "v2", "e3", "v3", "e1"];

    let actual: Vec<&String> = machine
        .profile
        .steps
        .iter()
        .map(|p| &p.element_id)
        .collect();

    assert_json_eq!(expected, actual);
}

#[test]
fn test_seed() {
    let mut machine = Machine::new();
    machine.seed(8739438725484);
    let index = fastrand::i32(0..1000);
    assert_eq!(index, 186);
    let index = fastrand::i32(0..1000);
    assert_eq!(index, 306);
    let index = fastrand::i32(0..1000);
    assert_eq!(index, 636);
    let index = fastrand::i32(0..1000);
    assert_eq!(index, 217);
}

#[test]
fn machine() {
    let mut machine = Machine::new();
    machine.seed(1234);
    assert!(machine
        .load_models(
            io::json_read::read(resource_path("login.json").to_str().unwrap())
                .expect("Expexted the test file to be loaded")
        )
        .is_ok());

    assert_eq!(machine.contexts.len(), 1);

    assert_eq!(machine.status, MachineStatus::NotStarted);

    let res = machine.walk();
    assert_eq!(
        res.is_ok(),
        true,
        "{:?}",
        Err::<(), Result<(), String>>(res)
    );

    let start_pos = machine.clone().start_pos;
    assert_eq!(start_pos.model_id, "login".to_string());

    let expected = vec![
        "n1", "e1", "n2", "e8", "n2", "e6", "n1", "e1", "n2", "e5", "n2", "e8", "n2", "e6", "n1",
        "e1", "n2", "e2", "n3", "e4", "n1", "e7", "n3", "e3",
    ];

    let actual: Vec<&String> = machine
        .profile
        .steps
        .iter()
        .map(|p| &p.element_id)
        .collect();

    assert_json_eq!(expected, actual);
}

#[test]
fn test_a_model() {
    let mut machine = Machine::new();
    assert!(machine
        .load_models(
            io::json_read::read(resource_path("petclinic.json").to_str().unwrap())
                .expect("Expexted the test file to be loaded")
        )
        .is_ok());

    let res = machine.walk();

    assert_eq!(
        res.is_ok(),
        true,
        "{:?}",
        Err::<(), Result<(), String>>(res)
    );
}
