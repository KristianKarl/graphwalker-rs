use std::collections::BTreeMap;
use std::sync::Arc;

use serde_derive::{Deserialize, Serialize};

// Deserialization used by example from https://github.com/serde-rs/serde/issues/936
mod models_to_hash {
    use super::Model;

    use std::collections::BTreeMap;
    use std::sync::Arc;

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::Serializer;

    pub fn serialize<S>(map: &BTreeMap<String, Arc<Model>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(map.values())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<String, Arc<Model>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = BTreeMap::new();
        for model in Vec::<Arc<Model>>::deserialize(deserializer)? {
            map.insert(model.id.clone(), model);
        }
        Ok(map)
    }
}
mod vertices_to_hash {
    use super::Vertex;

    use std::collections::BTreeMap;
    use std::sync::Arc;

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::Serializer;

    pub fn serialize<S>(
        map: &BTreeMap<String, Arc<Vertex>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(map.values())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<String, Arc<Vertex>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = BTreeMap::new();
        for vertex in Vec::<Arc<Vertex>>::deserialize(deserializer)? {
            map.insert(vertex.id.clone(), vertex);
        }
        Ok(map)
    }
}

mod edges_to_hash {
    use super::Edge;

    use std::collections::BTreeMap;
    use std::sync::Arc;

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::Serializer;

    pub fn serialize<S>(map: &BTreeMap<String, Arc<Edge>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(map.values())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<String, Arc<Edge>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = BTreeMap::new();
        for edge in Vec::<Arc<Edge>>::deserialize(deserializer)? {
            map.insert(edge.id.clone(), edge);
        }
        Ok(map)
    }
}

enum ElementKind {
    Model,
    Vertex,
    Edge,
}

trait Element {
    fn kind(&self) -> ElementKind;
    fn id(&self) -> &str;
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Models {
    pub start_element_id: Option<String>,

    #[serde(with = "models_to_hash")]
    pub models: BTreeMap<String, Arc<Model>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub name: Option<String>,

    #[serde(with = "vertices_to_hash")]
    pub vertices: BTreeMap<String, Arc<Vertex>>,

    #[serde(with = "edges_to_hash")]
    pub edges: BTreeMap<String, Arc<Edge>>,

    pub generator: Option<String>,
    #[serde(default)]
    pub actions: Vec<String>,
}

impl Element for Model {
    fn kind(&self) -> ElementKind {
        ElementKind::Model
    }

    fn id(&self) -> &str {
        self.id.as_str()
    }
}

impl Model {
    #[must_use]
    pub fn has_id(&self, id: String) -> bool {
        if self.edges.contains_key(&id) || self.vertices.contains_key(&id) {
            return true;
        }
        false
    }

    pub fn get_name_for_id(&self, id: &String) -> Option<String> {
        if let Some(e) = self.edges.get(id) {
            return e.name.clone();
        }
        if let Some(v) = self.vertices.get(id) {
            return v.name.clone();
        }
        None
    }

    pub fn out_edges(&self, id: String) -> Vec<Arc<Edge>> {
        let mut out_edges: Vec<Arc<Edge>> = Vec::new();
        for edge in self.edges.values() {
            if edge.source_vertex_id == id {
                out_edges.push(edge.clone());
            }
        }
        out_edges
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Vertex {
    pub id: String,
    pub name: Option<String>,
    pub shared_state: Option<String>,

    #[serde(default)]
    pub requirements: Vec<String>,

    #[serde(default)]
    pub actions: Vec<String>,
}

impl Element for Vertex {
    fn kind(&self) -> ElementKind {
        ElementKind::Vertex
    }

    fn id(&self) -> &str {
        self.id.as_str()
    }
}

impl Vertex {
    pub fn id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub id: String,
    pub name: Option<String>,

    #[serde(default)]
    pub actions: Vec<String>,

    #[serde(default)]
    pub requirements: Vec<String>,

    pub guard: Option<String>,

    pub source_vertex_id: String,
    pub target_vertex_id: String,
}

impl Element for Edge {
    fn kind(&self) -> ElementKind {
        ElementKind::Edge
    }

    fn id(&self) -> &str {
        self.id.as_str()
    }
}

impl Edge {
    pub fn id(mut self, id: &str, src: &str, dst: &str) -> Self {
        self.id = id.to_string();
        self.source_vertex_id = src.to_string();
        self.target_vertex_id = dst.to_string();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    fn create_model() -> Model {
        let mut model: Model = Model::default();
        model
            .vertices
            .insert("a".to_string(), Arc::new(Vertex::default().id("a")));
        model
            .vertices
            .insert("b".to_string(), Arc::new(Vertex::default().id("b")));
        model
            .vertices
            .insert("c".to_string(), Arc::new(Vertex::default().id("c")));
        model.edges.insert(
            "a->b".to_string(),
            Arc::new(Edge::default().id("a->b", "a", "a")),
        );
        model.edges.insert(
            "b->c".to_string(),
            Arc::new(Edge::default().id("b->c", "b", "c")),
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
        assert_eq!(v.id, "a");

        let v = model.vertices.get("x");
        assert!(v.is_none());
    }

    #[test]
    fn get_edge_test() {
        let model = create_model();
        let a = model.edges.get("b->c").unwrap();
        assert_eq!(a.id, "b->c");

        let b = model.edges.get("x");
        assert!(b.is_none());
    }

    #[test]
    fn has_id_test() {
        let model = create_model();
        assert!(model.has_id("a".to_string()));
        assert!(model.has_id("c".to_string()));
        assert!(model.has_id("b->c".to_string()));

        // Negative tests
        assert!(!model.has_id(String::new()));
        assert!(!model.has_id("x".to_string()));
    }

    #[test]
    fn serialize_vertex() {
        let vertex = Vertex::default();
        let vertex_json_str = serde_json::to_string_pretty(&vertex).unwrap();
        let v: Vertex = serde_json::from_str(vertex_json_str.as_str()).expect("Test failed");
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
        let v: Vertex = serde_json::from_str(vertex_json_str).expect("Test failed");
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
        let v: Vertex = serde_json::from_str(vertex_json_str).expect("Test failed");
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
