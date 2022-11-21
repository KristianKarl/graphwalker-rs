use serde_derive::{Deserialize,Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Models {
    pub models: Vec<Model>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    id: String,
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,

    #[serde(default)]
    pub generator: String,

    #[serde(default)]
    pub start_element_id: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Vertex {
    pub id: String,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub shared_state: String,

    #[serde(default)]
    pub requirements: Vec<String>,

    #[serde(default)]
    pub actions: Vec<String>,

    #[serde(default)]
    pub properties: Properties,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub id: String,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub actions: Vec<String>,

    #[serde(default)]
    pub requirements: Vec<String>,

    #[serde(default)]
    pub guard: String,

    #[serde(default)]
    pub properties: Properties,

    pub source_vertex_id: String,
    pub target_vertex_id: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Properties {
    #[serde(default)]
    pub x: f32,

    #[serde(default)]
    pub y: f32,

    #[serde(default)]
    pub description: String,
}

pub fn get_vertex_name(vertices: &Vec<Vertex>, id: &str) -> String {
    for vertex in vertices {
        if vertex.id == id {
            if vertex.name.is_empty() {
                return String::from("Start");
            }
            return String::from(&vertex.name);
        }
    }
    return String::from("");
}


pub fn get_vertex<'a>(vertices: &'a Vec<Vertex>, id: &'a String) -> Result<&'a Vertex, String> {
    for vertex in vertices {
        if vertex.id.eq(id) {
            return Ok(vertex);
        }
    }
    Err(format!("Vertex with id '{}', is not found.", id))
}
