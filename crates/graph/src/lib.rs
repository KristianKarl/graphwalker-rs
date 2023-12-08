use std::{
    collections::{BTreeMap, VecDeque},
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

    pub fn get_shortest_path(&self, start: &str, end: &str) -> Option<Vec<Arc<Edge>>> {
        if !self.has_id(start) {
            let msg = format!(
                "The start vertex: {} is not parth of the model: {}",
                start, self.id
            );
            log::error!("{}", msg);
            return None;
        }
        if !self.has_id(end) {
            let msg = format!(
                "The end vertex: {} is not parth of the model: {}",
                end, self.id
            );
            log::error!("{}", msg);
            return None;
        }

        let mut path: Vec<Arc<Edge>> = Vec::default();
        let mut visited: Vec<Arc<Vertex>> = Vec::default();
        let mut queue: VecDeque<Arc<Vertex>> = VecDeque::default();
        let binding = self.vertices.read().unwrap();
        let start_vertex = binding.get(start).unwrap();

        visited.push(Arc::clone(start_vertex));
        queue.push_back(Arc::clone(start_vertex));

        while let Some(v) = queue.pop_front() {
            if v.id == end {
                return Some(self.recreate_shortest_path(start, end, path));
            }

            for edge in self.out_edges(&v.id) {
                let binding = self.vertices.read().unwrap();
                let vertex = binding.get(&edge.target_vertex_id).unwrap();

                if !visited.contains(vertex) {
                    visited.push(Arc::clone(vertex));
                    queue.push_back(Arc::clone(vertex));
                    path.push(edge);
                }
            }
        }
        None
    }

    fn recreate_shortest_path(
        &self,
        start: &str,
        end: &str,
        path: Vec<Arc<Edge>>,
    ) -> Vec<Arc<Edge>> {
        let mut target: &str = end;
        let mut shortest_path: Vec<Arc<Edge>> = Vec::default();

        while !path.is_empty() {
            let pos = path
                .iter()
                .position(|e| e.target_vertex_id == target)
                .unwrap();

            if let Some(edge) = path.get(pos) {
                shortest_path.push(Arc::clone(edge));
                if edge.source_vertex_id == start {
                    break;
                } else {
                    target = &edge.source_vertex_id;
                }
            }
        }
        shortest_path.reverse();
        shortest_path
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
