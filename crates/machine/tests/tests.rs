use std::fs;

use assert_json_diff::assert_json_eq;
use machine::{Machine, MachineStatus};
use pretty_assertions::assert_eq;
use serde_json::Value;

fn resource_path(resource: &str) -> std::path::PathBuf {
    let mut path = std::path::PathBuf::new();
    path.push(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path.push("resources");
    path.push(resource);
    path
}

#[test]
fn test_seed() {
    let mut machine = Machine::default();
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
fn walk_multiple_model() {
    let mut machine = Machine::default();
    machine.seed(1234);
    let res = machine.load_models(
        io::json_read::read(
            resource_path("models/simpleMultiModel.json")
                .to_str()
                .unwrap(),
        )
        .unwrap(),
    );
    assert_eq!(res.is_ok(), true);

    let res = machine.walk();
    assert_eq!(
        res.is_ok(),
        true,
        "{:?}",
        Err::<(), Result<(), String>>(res)
    );

    let res = fs::read_to_string(
        resource_path("results/simpleMultiModel_1234.json")
            .to_str()
            .unwrap(),
    )
    .unwrap();
    let expected: Vec<Value> = serde_json::from_str(res.as_str()).unwrap();

    let actual: Vec<Value> = machine
        .profile
        .steps
        .iter()
        .map(|step| serde_json::to_value(&step).unwrap())
        .collect();

    assert_json_eq!(expected, actual);
}

#[test]
fn walk_single_model() {
    let mut machine = Machine::default();
    machine.seed(1234);
    let res = machine.load_models(
        io::json_read::read(
            resource_path("models/simpleSingleModel.json")
                .to_str()
                .unwrap(),
        )
        .unwrap(),
    );
    assert_eq!(res.is_ok(), true);

    let res = machine.walk();
    assert_eq!(
        res.is_ok(),
        true,
        "{:?}",
        Err::<(), Result<(), String>>(res)
    );

    let res = fs::read_to_string(
        resource_path("results/simpleSingleModel_1234.json")
            .to_str()
            .unwrap(),
    )
    .unwrap();
    let expected: Vec<Value> = serde_json::from_str(res.as_str()).unwrap();

    let actual: Vec<Value> = machine
        .profile
        .steps
        .iter()
        .map(|step| serde_json::to_value(&step).unwrap())
        .collect();

    assert_json_eq!(expected, actual);
}

#[test]
fn machine() {
    let mut machine = Machine::default();
    machine.seed(1234);
    assert!(machine
        .load_models(
            io::json_read::read(resource_path("models/login.json").to_str().unwrap()).unwrap()
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

    let res =
        fs::read_to_string(resource_path("results/login_1234.json").to_str().unwrap()).unwrap();
    let expected: Vec<Value> = serde_json::from_str(res.as_str()).unwrap();

    let actual: Vec<Value> = machine
        .profile
        .steps
        .iter()
        .map(|step| serde_json::to_value(&step).unwrap())
        .collect();

    assert_json_eq!(expected, actual);
}

#[test]
fn test_a_model() {
    let mut machine = Machine::default();
    machine.seed(1234);
    assert!(machine
        .load_models(
            io::json_read::read(resource_path("models/petclinic.json").to_str().unwrap()).unwrap()
        )
        .is_ok());

    let res = machine.walk();

    assert_eq!(
        res.is_ok(),
        true,
        "{:?}",
        Err::<(), Result<(), String>>(res)
    );

    let res = fs::read_to_string(
        resource_path("results/petclinic_1234.json")
            .to_str()
            .unwrap(),
    )
    .unwrap();
    let expected: Vec<Value> = serde_json::from_str(res.as_str()).unwrap();

    let actual: Vec<Value> = machine
        .profile
        .steps
        .iter()
        .map(|step| serde_json::to_value(&step).unwrap())
        .collect();

    assert_json_eq!(expected, actual);
}
