use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use serde_derive::{Deserialize, Serialize};

// Deserialization used by example from https://github.com/serde-rs/serde/issues/936
mod models_to_hash {
    use super::Model;

    use std::collections::BTreeMap;
    use std::sync::Arc;

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::Serializer;

    pub fn serialize<S>(
        map: &Arc<BTreeMap<String, Model>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(map.values())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<BTreeMap<String, Model>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = BTreeMap::new();
        for model in Vec::<Model>::deserialize(deserializer)? {
            map.insert(model.id.clone(), model);
        }
        Ok(Arc::new(map))
    }
}
mod vertices_to_hash {
    use super::Vertex;

    use std::collections::BTreeMap;
    use std::sync::{Arc, RwLock};

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::Serializer;

    pub fn serialize<S>(
        map: &Arc<RwLock<BTreeMap<String, Arc<Vertex>>>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(map.read().unwrap().values())
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Arc<RwLock<BTreeMap<String, Arc<Vertex>>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = BTreeMap::new();
        for vertex in Vec::<Vertex>::deserialize(deserializer)? {
            map.insert(vertex.id.clone(), Arc::new(vertex));
        }
        Ok(Arc::new(RwLock::new(map)))
    }
}

mod edges_to_hash {
    use super::Edge;

    use std::collections::BTreeMap;
    use std::sync::{Arc, RwLock};

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::Serializer;

    pub fn serialize<S>(
        map: &Arc<RwLock<BTreeMap<String, Arc<Edge>>>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(map.read().unwrap().values())
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Arc<RwLock<BTreeMap<String, Arc<Edge>>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = BTreeMap::new();
        for edge in Vec::<Edge>::deserialize(deserializer)? {
            map.insert(edge.id.clone(), Arc::new(edge));
        }
        Ok(Arc::new(RwLock::new(map)))
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
    pub models: Arc<BTreeMap<String, Model>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub name: Option<String>,

    #[serde(with = "vertices_to_hash")]
    pub vertices: Arc<RwLock<BTreeMap<String, Arc<Vertex>>>>,

    #[serde(with = "edges_to_hash")]
    pub edges: Arc<RwLock<BTreeMap<String, Arc<Edge>>>>,

    pub generator: Option<String>,
    #[serde(default)]
    pub actions: Vec<String>,
}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Model {}

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
    pub fn has_id(&self, id: &str) -> bool {
        if self.edges.read().unwrap().contains_key(id)
            || self.vertices.read().unwrap().contains_key(id)
        {
            return true;
        }
        false
    }

    pub fn get_name_for_id(&self, id: &String) -> Option<String> {
        if let Some(e) = self.edges.read().unwrap().get(id) {
            return e.name.clone();
        }
        if let Some(v) = self.vertices.read().unwrap().get(id) {
            return v.name.clone();
        }
        None
    }

    pub fn out_edges(&self, id: &str) -> Vec<Arc<Edge>> {
        let mut out_edges: Vec<Arc<Edge>> = Vec::new();
        for edge in self.edges.read().unwrap().values() {
            if edge.source_vertex_id == *id {
                out_edges.push(Arc::clone(edge));
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
