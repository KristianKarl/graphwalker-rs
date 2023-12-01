use std::{fs, sync::Arc};

use assert_json_diff::assert_json_eq;
use machine::{Machine, MachineStatus, Position};
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

#[derive(Debug)]
pub struct TestData {
    model_file: String,
    expected_file: String,
    seed: u64,
    ignore_guards: bool,
    number_of_models: usize,
    start_pos: Position,
}

fn get_test_data() -> Vec<TestData> {
    vec![
        TestData {
            model_file: "models/login.json".to_string(),
            expected_file: "results/login_ignore_guards_870151202047466989.json".to_string(),
            seed: 870151202047466989,
            ignore_guards: true,
            number_of_models: 1,
            start_pos: Position {
                model_id: Arc::new("".to_string()),
                element_id: Arc::new("n1".to_string()),
            },
        },
        TestData {
            model_file: "models/simpleSingleModel.json".to_string(),
            expected_file: "results/simpleSingleModel_1234.json".to_string(),
            seed: 1234,
            ignore_guards: false,
            number_of_models: 1,
            start_pos: Position {
                model_id: Arc::new("".to_string()),
                element_id: Arc::new("v1".to_string()),
            },
        },
        TestData {
            model_file: "models/simpleMultiModel.json".to_string(),
            expected_file: "results/simpleMultiModel_1234.json".to_string(),
            seed: 1234,
            ignore_guards: false,
            number_of_models: 2,
            start_pos: Position {
                model_id: Arc::new("".to_string()),
                element_id: Arc::new("v11".to_string()),
            },
        },
        TestData {
            model_file: "models/login.json".to_string(),
            expected_file: "results/login_1234.json".to_string(),
            seed: 1234,
            ignore_guards: false,
            number_of_models: 1,
            start_pos: Position {
                model_id: Arc::new("".to_string()),
                element_id: Arc::new("n1".to_string()),
            },
        },
        TestData {
            model_file: "models/petclinic.json".to_string(),
            expected_file: "results/petclinic_1234.json".to_string(),
            seed: 1234,
            ignore_guards: false,
            number_of_models: 5,
            start_pos: Position {
                model_id: Arc::new("".to_string()),
                element_id: Arc::new("32ea3d10-789a-11ea-8c87-010078a2bc20".to_string()),
            },
        },
    ]
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
fn verify_all_test_models() {
    for data in get_test_data() {
        dbg!(&data);
        let mut machine = Machine::default();
        machine.seed(data.seed);
        assert!(machine
            .load_models(
                io::json_read::read(resource_path(data.model_file.as_str()).to_str().unwrap())
                    .unwrap()
            )
            .is_ok());

        assert_eq!(machine.contexts.len(), data.number_of_models);
        assert_eq!(machine.status, MachineStatus::NotStarted);
        assert_eq!(machine.clone().start_pos, data.start_pos);

        machine.ignore_guards = data.ignore_guards;
        let res = machine.walk();
        assert_eq!(
            res.is_ok(),
            true,
            "{:?}",
            Err::<(), Result<(), String>>(res)
        );

        let res = fs::read_to_string(resource_path(data.expected_file.as_str()).to_str().unwrap())
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
}
