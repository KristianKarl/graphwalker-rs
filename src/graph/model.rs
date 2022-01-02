use serde_derive::{ Deserialize,Serialize };

#[derive(Serialize, Deserialize, Debug)]
pub struct Models {
    pub models: Vec<Model>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    id: String,
    name: String,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,

    #[serde(default)]
    generator: String,

    #[serde(default)]
    start_element_id: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Vertex {
    id: String,

    #[serde(default)]
    name: String,

    #[serde(default)]
    shared_state: String,

    #[serde(default)]
    requirements: Vec<String>,

    #[serde(default)]
    actions: Vec<String>,

    #[serde(default)]
    properties: Properties,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    id: String,

    #[serde(default)]
    name: String,

    #[serde(default)]
    actions:Vec<String>,

    #[serde(default)]
    requirements: Vec<String>,

    #[serde(default)]
    properties: Properties,

    pub source_vertex_id: String,
    pub target_vertex_id: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Properties {
    #[serde(default)]
    x: f32,

    #[serde(default)]
    y: f32,

    #[serde(default)]
    description: String,
}