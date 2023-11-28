use std::collections::BTreeMap;

use serde_derive::{Deserialize, Serialize};

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
    pub models: BTreeMap<String, Model>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub name: Option<String>,

    #[serde(with = "vertices_to_hash")]
    pub vertices: BTreeMap<String, Vertex>,

    #[serde(with = "edges_to_hash")]
    pub edges: BTreeMap<String, Edge>,

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

    pub fn out_edges(&self, id: String) -> Vec<Edge> {
        let mut out_edges: Vec<Edge> = Vec::new();
        for edge in self.edges.values() {
            if edge.source_vertex_id == id {
                out_edges.push(edge.clone());
            }
        }
        out_edges
    }
}

// Deserialization used by example from https://github.com/serde-rs/serde/issues/936
mod models_to_hash {
    use super::Model;

    use std::collections::BTreeMap;

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::Serializer;

    pub fn serialize<S>(map: &BTreeMap<String, Model>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(map.values())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<String, Model>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = BTreeMap::new();
        for model in Vec::<Model>::deserialize(deserializer)? {
            map.insert(model.id.clone(), model);
        }
        Ok(map)
    }
}
mod vertices_to_hash {
    use super::Vertex;

    use std::collections::BTreeMap;

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::Serializer;

    pub fn serialize<S>(
        map: &BTreeMap<String, Vertex>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(map.values())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<String, Vertex>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = BTreeMap::new();
        for vertex in Vec::<Vertex>::deserialize(deserializer)? {
            map.insert(vertex.id.clone(), vertex);
        }
        Ok(map)
    }
}

mod edges_to_hash {
    use super::Edge;

    use std::collections::BTreeMap;

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::Serializer;

    pub fn serialize<S>(map: &BTreeMap<String, Edge>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(map.values())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<String, Edge>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = BTreeMap::new();
        for edge in Vec::<Edge>::deserialize(deserializer)? {
            map.insert(edge.id.clone(), edge);
        }
        Ok(map)
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

