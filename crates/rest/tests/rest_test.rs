use std::fs;

use assert_json_diff::assert_json_eq;
use machine::Machine;
use rest::{init_machine, routes};
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

#[tokio::test]
async fn get_next_bad_request() {
    let m = init_machine(Machine::default());
    let graphwalker_routes = routes::graphwalker_routes(m);

    let res = warp::test::request() // 2.
        .method("GET")
        .path("/hasNext")
        .reply(&graphwalker_routes)
        .await;

    assert_eq!(res.status(), 200, "Should return 200 OK.");
    assert_eq!(res.body(), "false", "Should return false.");
}

#[tokio::test]
async fn login_model() {
    let file_read_result = io::read(resource_path("models/login.json").to_str().unwrap());
    let models = match file_read_result {
        Ok(models) => models,
        Err(error) => {
            panic!(
                "File login.json did make sense rto read for graphwalker {}",
                error
            )
        }
    };

    let mut machine = machine::Machine::default();
    machine.seed(1234);
    let res = machine.load_models(models);
    if res.is_err() {
        panic!(
            "Loading the models into a Machine failed {}",
            res.err().expect("An error message")
        )
    }

    assert_eq!(machine.reset().is_ok(), true);

    let m = init_machine(machine);
    let graphwalker_routes = routes::graphwalker_routes(m);

    let res = warp::test::request()
        .method("GET")
        .path("/hasNext")
        .reply(&graphwalker_routes)
        .await;
    assert_eq!(res.status(), 200, "Should return 200 OK.");
    assert_eq!(res.body(), "true", "Should return true.");

    let res =
        fs::read_to_string(resource_path("results/login_1234.json").to_str().unwrap()).unwrap();
    let expected: Vec<Value> = serde_json::from_str(res.as_str()).unwrap();
    let mut actual: Vec<Value> = vec![];

    for _element in expected.clone() {
        let res = warp::test::request()
            .method("GET")
            .path("/hasNext")
            .reply(&graphwalker_routes)
            .await;
        assert_eq!(res.status(), 200, "Should return 200 OK.");
        assert_eq!(res.body(), "true", "Should return true.");

        let res = warp::test::request()
            .method("GET")
            .path("/getNext")
            .reply(&graphwalker_routes)
            .await;
        assert_eq!(res.status(), 200, "Should return 200 OK.");

        //let body = serde_json::from_str(std::str::from_utf8(res.body()).unwrap()).unwrap();
        let body = serde_json::from_str(std::str::from_utf8(res.body()).unwrap()).unwrap();
        actual.push(body);
    }

    assert_json_eq!(expected, actual);

    let res = warp::test::request()
        .method("GET")
        .path("/hasNext")
        .reply(&graphwalker_routes)
        .await;
    assert_eq!(res.status(), 200, "Should return 200 OK.");
    assert_eq!(res.body(), "false", "Should return false.");
}
