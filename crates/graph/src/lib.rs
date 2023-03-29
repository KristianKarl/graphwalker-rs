use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

// Deserialization used by example from https://github.com/serde-rs/serde/issues/936
mod models_to_hash {
    use super::Model;

    use std::collections::HashMap;

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::Serializer;

    pub fn serialize<S>(map: &HashMap<String, Model>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(map.values())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<String, Model>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = HashMap::new();
        for model in Vec::<Model>::deserialize(deserializer)? {
            map.insert(model.id.clone().unwrap(), model);
        }
        Ok(map)
    }
}
mod vertices_to_hash {
    use super::Vertex;

    use std::collections::HashMap;

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::Serializer;

    pub fn serialize<S>(map: &HashMap<String, Vertex>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(map.values())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<String, Vertex>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = HashMap::new();
        for vertex in Vec::<Vertex>::deserialize(deserializer)? {
            map.insert(vertex.id.clone().unwrap(), vertex);
        }
        Ok(map)
    }
}

mod edges_to_hash {
    use super::Edge;

    use std::collections::HashMap;

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::Serializer;

    pub fn serialize<S>(map: &HashMap<String, Edge>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(map.values())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<String, Edge>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = HashMap::new();
        for edge in Vec::<Edge>::deserialize(deserializer)? {
            map.insert(edge.id.clone().unwrap(), edge);
        }
        Ok(map)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Models {
    #[serde(with = "models_to_hash")]
    pub models: HashMap<String, Model>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: Option<String>,
    pub name: Option<String>,

    #[serde(with = "vertices_to_hash")]
    pub vertices: HashMap<String, Vertex>,

    #[serde(with = "edges_to_hash")]
    pub edges: HashMap<String, Edge>,

    pub generator: Option<String>,
    pub start_element_id: Option<String>,
}

impl Model {
    pub fn new() -> Model {
        Model {
            id: None,
            name: None,
            vertices: HashMap::new(),
            edges: HashMap::new(),
            generator: None,
            start_element_id: None,
        }
    }

    pub fn has_id(&self, id: Option<String>) -> bool {
        match id {
            Some(i) => {
                if self.edges.contains_key(&i) {
                    return true;
                } else if self.vertices.contains_key(&i) {
                    return true;
                }
                return false;
            }
            None => return false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Vertex {
    pub id: Option<String>,
    pub name: Option<String>,
    pub shared_state: Option<String>,

    #[serde(default)]
    pub requirements: Vec<String>,

    #[serde(default)]
    pub actions: Vec<String>,
}

impl Vertex {
    pub fn new() -> Vertex {
        Vertex {
            id: None,
            name: None,
            shared_state: None,
            requirements: vec![],
            actions: vec![],
        }
    }

    pub fn id(mut self, id: String) -> Vertex {
        self.id = Some(id);
        self
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub id: Option<String>,
    pub name: Option<String>,

    #[serde(default)]
    pub actions: Vec<String>,

    #[serde(default)]
    pub requirements: Vec<String>,

    pub guard: Option<String>,

    pub source_vertex_id: Option<String>,
    pub target_vertex_id: Option<String>,
}

impl Edge {
    pub fn new() -> Edge {
        Edge {
            id: None,
            name: None,
            source_vertex_id: None,
            target_vertex_id: None,
            guard: None,
            requirements: vec![],
            actions: vec![],
        }
    }

    pub fn id(mut self, id: String) -> Edge {
        self.id = Some(id);
        self
    }

    pub fn source_vertex_id(mut self, id: String) -> Edge {
        self.source_vertex_id = Some(id);
        self
    }

    pub fn target_vertex_id(mut self, id: String) -> Edge {
        self.target_vertex_id = Some(id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    fn create_model() -> Model {
        let mut model: Model = Model::new();
        model
            .vertices
            .insert("a".to_string(), Vertex::new().id("a".to_string()));
        model
            .vertices
            .insert("b".to_string(), Vertex::new().id("b".to_string()));
        model
            .vertices
            .insert("c".to_string(), Vertex::new().id("c".to_string()));
        model.edges.insert(
            "a->b".to_string(),
            Edge::new()
                .id("a->b".to_string())
                .source_vertex_id("a".to_string())
                .target_vertex_id("a".to_string()),
        );
        model.edges.insert(
            "b->c".to_string(),
            Edge::new()
                .id("b->c".to_string())
                .source_vertex_id("b".to_string())
                .target_vertex_id("c".to_string()),
        );
        model
    }

    #[test]
    fn build_model_test() {
        let model = create_model();
        assert_eq!(model.vertices.len(), 3);
        assert_eq!(model.edges.len(), 2);
    }

    #[test]
    fn get_vertex_test() {
        let model = create_model();
        let v = model.vertices.get("a").unwrap();
        assert_eq!(v.id.clone().unwrap(), "a");

        let v = model.vertices.get("x");
        assert!(v.is_none());
    }

    #[test]
    fn get_edge_test() {
        let model = create_model();
        let a = model.edges.get("b->c").unwrap();
        assert_eq!(a.id.clone().unwrap(), "b->c");

        let b = model.edges.get("x");
        assert!(b.is_none());
    }

    #[test]
    fn has_id_test() {
        let model = create_model();
        assert!(model.has_id(Some("a".to_string())));
        assert!(model.has_id(Some("c".to_string())));
        assert!(model.has_id(Some("b->c".to_string())));

        // Negative tests
        assert!(!model.has_id(None));
        assert!(!model.has_id(Some("x".to_string())));
    }

    #[test]
    fn serialize_vertex() {
        let vertex = Vertex::new();
        let vertex_json_str = serde_json::to_string_pretty(&vertex).unwrap();
        let v: Vertex = serde_json::from_str(vertex_json_str.as_str()).expect("Test failed");
        assert!(v.id.is_none());
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
        let v: Vertex = serde_json::from_str(vertex_json_str).expect("Test failed");
        assert_eq!(v.id.unwrap(), "n1");
        assert_eq!(v.name.unwrap(), "v_ClientNotRunning");
        assert_eq!(v.shared_state.unwrap(), "CLIENT_NOT_RUNNNG");
        assert!(v.actions.is_empty());
        assert!(v.requirements.is_empty());

        let vertex_json_str = r#"
        {
            "id": "n1",
            "name": "v_ClientNotRunning"
        }"#;
        let v: Vertex = serde_json::from_str(vertex_json_str).expect("Test failed");
        assert_eq!(v.id.unwrap(), "n1");
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
        let m: Models = serde_json::from_str(vertex_json_str).expect("Test failed");
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
        let m: Models = serde_json::from_str(vertex_json_str).expect("Test failed");
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
        let models: Models = serde_json::from_str(models_json_str).expect("Test failed");
        let models_json_str_serialized = serde_json::to_string_pretty(&models).unwrap();
        let deserialized_models: Models =
            serde_json::from_str(&models_json_str_serialized).expect("Test failed");
        assert_eq!(models, deserialized_models);
    }
}
