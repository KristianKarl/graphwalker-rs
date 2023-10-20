use assert_json_diff::assert_json_eq;
use machine::{Machine, Position};
use rest::{init_machine, routes};
use snailquote::unescape;

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

#[tokio::test]
async fn get_next_bad_request() {
    let m = init_machine(Machine::new());
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
    let file_read_result = io::read(
        resource_path("login.json")
            .to_str()
            .expect("The login.json file to be readable"),
    );
    let models = match file_read_result {
        Ok(models) => models,
        Err(error) => {
            panic!(
                "File login.json did make sense rto read for graphwalker {}",
                error
            )
        }
    };

    let mut machine = machine::Machine::new();
    machine.seed(8739438725484);
    let res = machine.load_models(models);
    if res.is_err() {
        panic!(
            "Loading the models into a Machine failed {}",
            res.err().expect("An error message")
        )
    }
    machine.reset();

    let m = init_machine(machine);
    let graphwalker_routes = routes::graphwalker_routes(m);

    let res = warp::test::request()
        .method("GET")
        .path("/hasNext")
        .reply(&graphwalker_routes)
        .await;
    assert_eq!(res.status(), 200, "Should return 200 OK.");
    assert_eq!(res.body(), "true", "Should return true.");

    let expected = vec![
        "e0", "n1", "e7", "n3", "e3", "n2", "e6", "n1", "e0", "n1", "e7", "n3", "e4", "n1", "e0",
        "n1", "e7", "n3", "e4", "n1", "e7", "n3", "e3", "n2", "e6", "n1", "e7", "n3", "e4", "n1",
        "e0", "n1", "e1", "n2", "e6", "n1", "e7", "n3", "e3", "n2", "e2", "n3", "e3", "n2", "e2",
        "n3", "e4", "n1", "e1", "n2", "e6", "n1", "e0", "n1", "e0", "n1", "e7", "n3", "e4", "n1",
        "e0", "n1", "e1", "n2", "e6", "n1", "e7", "n3", "e4", "n1", "e1", "n2", "e2", "n3", "e4",
        "n1", "e0", "n1", "e1", "n2", "e6", "n1", "e1", "n2", "e2", "n3", "e3", "n2", "e8", "n2",
        "e8", "n2", "e2", "n3", "e4", "n1", "e0", "n1", "e7", "n3", "e3", "n2", "e2", "n3", "e3",
        "n2", "e8", "n2", "e5",
    ];

    for element in expected {
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

        let expected = Position {
            model_id: "853429e2-0528-48b9-97b3-7725eafbb8b5".to_string(),
            element_id: element.to_string(),
        };
        let body = std::str::from_utf8(res.body()).expect("Found invalid UTF-8");
        assert_json_eq!(
            unescape(body).unwrap(),
            serde_json::to_string(&expected).unwrap()
        );
    }
    let res = warp::test::request()
        .method("GET")
        .path("/hasNext")
        .reply(&graphwalker_routes)
        .await;
    assert_eq!(res.status(), 200, "Should return 200 OK.");
    assert_eq!(res.body(), "false", "Should return false.");
}
