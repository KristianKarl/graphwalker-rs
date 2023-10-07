// https://github.com/andrewleverette/rust_warp_api/tree/master

mod handlers;
mod routes;

use machine::Machine;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type MachineRt = Arc<Mutex<Machine>>;

pub fn init_machine(machine: Machine) -> MachineRt {
    Arc::new(Mutex::new(machine))
}

#[tokio::main]
pub async fn run_rest_service(machine: Machine) {
    let m = init_machine(machine);
    let graphwalker_routes = routes::graphwalker_routes(m);

    warp::serve(graphwalker_routes)
        .run(([127, 0, 0, 1], 3000))
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_json::assert_json;

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

        assert_eq!(res.status(), 400, "Should return 400 Bad request, since the machine is not loaded with anything meaningful");
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

        let m = init_machine(machine);
        let graphwalker_routes = routes::graphwalker_routes(m);

        let res = warp::test::request()
            .method("GET")
            .path("/hasNext")
            .reply(&graphwalker_routes)
            .await;
        assert_eq!(res.status(), 200, "Should return 200 OK.");
        assert_eq!(res.body(), "true", "Should return true.");

        let expected_path = vec![
            "e0", "n1", "n1", "e1", "n2", "e6", "n1", "e1", "n2", "e6", "n1", "e0", "n1", "e1",
            "n2", "e2", "n3", "e4", "n1", "e7", "n3", "e3", "n2", "e6", "n1", "e0", "n1", "n1",
            "n1", "e7", "n3", "n3", "n3", "e3", "n2", "e8", "n2", "e5",
        ];

        for element in expected_path {
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
            let body = std::str::from_utf8(res.body()).expect("Found invalid UTF-8");
            assert_json!( body, {
                "context_id": "853429e2-0528-48b9-97b3-7725eafbb8b5",
                "element_id": element
                }
            );
        }
        let res = warp::test::request()
            .method("GET")
            .path("/hasNext")
            .reply(&graphwalker_routes)
            .await;
        assert_eq!(res.status(), 200, "Should return 200 OK.");
        assert_eq!(res.body(), "false", "Should return true.");
    }
}
