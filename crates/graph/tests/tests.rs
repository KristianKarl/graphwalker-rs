use std::sync::Arc;

use graph::{Edge, Model, Models, Vertex};
use pretty_assertions::assert_eq;

fn create_model() -> Model {
    let model: Model = Model::default();
    let vertices = Arc::clone(&model.vertices);
    let edges = Arc::clone(&model.edges);

    vertices
        .write()
        .unwrap()
        .insert("a".to_string(), Arc::new(Vertex::default().id("a")));
    vertices
        .write()
        .unwrap()
        .insert("b".to_string(), Arc::new(Vertex::default().id("b")));
    vertices
        .write()
        .unwrap()
        .insert("c".to_string(), Arc::new(Vertex::default().id("c")));
    edges.write().unwrap().insert(
        "a->b".to_string(),
        Arc::new(Edge::default().id("a->b", "a", "a")),
    );
    edges.write().unwrap().insert(
        "b->c".to_string(),
        Arc::new(Edge::default().id("b->c", "b", "c")),
    );
    model
}

#[test]
fn build_model_test() {
    let model = create_model();
    assert_eq!(model.vertices.read().unwrap().len(), 3);
    assert_eq!(model.edges.read().unwrap().len(), 2);
}

#[test]
fn get_vertex_test() {
    let model = create_model();
    let binding = model.vertices.read().unwrap();
    let v = binding.get("a");
    assert_eq!(v.unwrap().id, "a");

    let binding = model.vertices.read().unwrap();
    let v = binding.get("x");
    assert!(v.is_none());
}

#[test]
fn get_edge_test() {
    let model = create_model();
    let binding = model.edges.read().unwrap();
    let a = binding.get("b->c");
    assert_eq!(a.unwrap().id, "b->c");

    let binding = model.edges.read().unwrap();
    let b = binding.get("x");
    assert!(b.is_none());
}

#[test]
fn has_id_test() {
    let model = create_model();
    assert!(model.has_id("a"));
    assert!(model.has_id("c"));
    assert!(model.has_id("b->c"));

    // Negative tests
    assert!(!model.has_id(""));
    assert!(!model.has_id("x"));
}

#[test]
fn serialize_vertex() {
    let vertex = Vertex::default();
    let vertex_json_str = serde_json::to_string_pretty(&vertex).unwrap();
    let v: Vertex = serde_json::from_str(vertex_json_str.as_str()).unwrap();
    assert!(v.id.is_empty());
    assert!(v.name.is_none());
    assert!(v.requirements.is_empty());
    assert!(v.actions.is_empty());
    assert!(v.requirements.is_empty());
}

#[test]
fn deserialize_vertex() {
    let vertex_json_str = r#"
        {
            "id": "n1",
            "name": "v_ClientNotRunning",
            "sharedState": "CLIENT_NOT_RUNNNG",
            "actions": [],
            "requirements": []
        }"#;
    let v: Vertex = serde_json::from_str(vertex_json_str).unwrap();
    assert_eq!(v.id, "n1");
    assert_eq!(v.name.unwrap(), "v_ClientNotRunning");
    assert_eq!(v.shared_state.unwrap(), "CLIENT_NOT_RUNNNG");
    assert!(v.actions.is_empty());
    assert!(v.requirements.is_empty());

    let vertex_json_str = r#"
        {
            "id": "n1",
            "name": "v_ClientNotRunning"
        }"#;
    let v: Vertex = serde_json::from_str(vertex_json_str).unwrap();
    assert_eq!(v.id, "n1");
    assert_eq!(v.name.unwrap(), "v_ClientNotRunning");
    assert!(v.shared_state.is_none());
    assert!(v.actions.is_empty());
    assert!(v.requirements.is_empty());
}

#[test]
fn deserialize_models() {
    let vertex_json_str = r#"
        {
            "models": []
        }"#;
    let m: Models = serde_json::from_str(vertex_json_str).unwrap();
    assert!(m.models.is_empty());

    let vertex_json_str = r#"
        {
            "models": [
              {
                "id": "853429e2-0528-48b9-97b3-7725eafbb8b5",
                "name": "Login",
                "vertices": [],
                "edges": []
              }
            ]
          }"#;
    let m: Models = serde_json::from_str(vertex_json_str).unwrap();
    assert_eq!(m.models.len(), 1);
}

#[test]
fn deserialize_login_model() {
    let models_json_str = r#"
        {
            "models": [
              {
                "id": "853429e2-0528-48b9-97b3-7725eafbb8b5",
                "name": "Login",
                "vertices": [
                  {
                    "id": "n1",
                    "name": "v_ClientNotRunning",
                    "sharedState": "CLIENT_NOT_RUNNNG",
                    "requirements": [],
                    "actions": []
                  },
                  {
                    "id": "n2",
                    "name": "v_LoginPrompted",
                    "sharedState": null,
                    "requirements": [],
                    "actions": []
                  },
                  {
                    "id": "n3",
                    "name": "v_Browse",
                    "sharedState": "LOGGED_IN",
                    "requirements": [],
                    "actions": []
                  },
                  {
                    "id": "Start",
                    "name": null,
                    "sharedState": null,
                    "requirements": [],
                    "actions": []
                  }
                ],
                "edges": [
                  {
                    "id": "e0",
                    "name": "e_Init",
                    "actions": [
                      "validLogin=false;rememberMe=false;"
                    ],
                    "requirements": [],
                    "guard": null,
                    "sourceVertexId": "Start",
                    "targetVertexId": "n1"
                  },
                  {
                    "id": "e1",
                    "name": "e_StartClient",
                    "actions": [],
                    "requirements": [],
                    "guard": "!rememberMe||!validLogin",
                    "sourceVertexId": "n1",
                    "targetVertexId": "n2"
                  },
                  {
                    "id": "e2",
                    "name": "e_ValidPremiumCredentials",
                    "actions": [
                      "validLogin=true;"
                    ],
                    "requirements": [],
                    "guard": null,
                    "sourceVertexId": "n2",
                    "targetVertexId": "n3"
                  },
                  {
                    "id": "e3",
                    "name": "e_Logout",
                    "actions": [],
                    "requirements": [],
                    "guard": null,
                    "sourceVertexId": "n3",
                    "targetVertexId": "n2"
                  },
                  {
                    "id": "e4",
                    "name": "e_Exit",
                    "actions": [],
                    "requirements": [],
                    "guard": null,
                    "sourceVertexId": "n3",
                    "targetVertexId": "n1"
                  },
                  {
                    "id": "e5",
                    "name": "e_ToggleRememberMe",
                    "actions": [
                      "rememberMe=!rememberMe;"
                    ],
                    "requirements": [],
                    "guard": null,
                    "sourceVertexId": "n2",
                    "targetVertexId": "n2"
                  },
                  {
                    "id": "e6",
                    "name": "e_Close",
                    "actions": [],
                    "requirements": [],
                    "guard": null,
                    "sourceVertexId": "n2",
                    "targetVertexId": "n1"
                  },
                  {
                    "id": "e7",
                    "name": "e_StartClient",
                    "actions": [],
                    "requirements": [],
                    "guard": "rememberMe&&validLogin",
                    "sourceVertexId": "n1",
                    "targetVertexId": "n3"
                  },
                  {
                    "id": "e8",
                    "name": "e_InvalidCredentials",
                    "actions": [
                      "validLogin=false;"
                    ],
                    "requirements": [],
                    "guard": null,
                    "sourceVertexId": "n2",
                    "targetVertexId": "n2"
                  }
                ],
                "generator": "random(edge_coverage(100))",
                "startElementId": "e0"
              }
            ]
          }"#;
    let models: Models = serde_json::from_str(models_json_str).unwrap();
    let models_json_str_serialized = serde_json::to_string_pretty(&models).unwrap();
    let deserialized_models: Models = serde_json::from_str(&models_json_str_serialized).unwrap();
    assert_eq!(models, deserialized_models);
}
