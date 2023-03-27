use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Models {
    pub models: Vec<Model>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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

impl Model {
    fn new() -> Model {
        Model {
            id: "".to_string(),
            name: "".to_string(),
            vertices: vec![],
            edges: vec![],
            generator: "".to_string(),
            start_element_id: "".to_string(),
        }
    }

    fn get_vertex(&self, id: String) -> Result<Vertex, String> {
        for vertex in self.vertices.iter() {
            if vertex.id == id {
                return Ok(vertex.clone());
            }
        }
        Err(format!("Vertex with id '{}', is not found.", id))
    }

    fn get_edge(&self, id: String) -> Result<Edge, String> {
        for edge in self.edges.iter() {
            if edge.id == id {
                return Ok(edge.clone());
            }
        }
        Err(format!("Edge with id '{}', is not found.", id))
    }
}


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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

impl Vertex {
    fn new() -> Vertex {
        Vertex {
            id: "".to_string(),
            name: "".to_string(),
            shared_state: "".to_string(),
            requirements: vec![],
            actions: vec![],
            properties: Properties::new(),
        }
    }

    fn id(mut self, id: String) -> Vertex {
        self.id = id;
        self
    }
}


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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

impl Edge {
    fn new() -> Edge {
        Edge {
            id: "".to_string(),
            name: "".to_string(),
            source_vertex_id: "".to_string(),
            target_vertex_id: "".to_string(),
            guard: "".to_string(),
            requirements: vec![],
            actions: vec![],
            properties: Properties::new(),
         }
    }

    fn id(mut self, id: String) -> Edge {
        self.id = id;
        self
    }

    fn source_vertex_id(mut self, id: String) -> Edge {
        self.source_vertex_id = id;
        self
    }

    fn target_vertex_id(mut self, id: String) -> Edge {
        self.target_vertex_id = id;
        self
    }
}


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Properties {
    #[serde(default)]
    pub x: f32,

    #[serde(default)]
    pub y: f32,

    #[serde(default)]
    pub description: String,
}

impl Properties {
    fn new() -> Properties {
        Properties {
            x: 0f32,
            y: 0f32,
            description: "".to_string()    
        }
    }
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


#[cfg(test)]
mod tests {
    use super::*;

    fn create_model() -> Model {
        let mut model: Model = Model::new();
        model.vertices.push(Vertex::new().id("a".to_string()));
        model.vertices.push(Vertex::new().id("b".to_string()));
        model.vertices.push(Vertex::new().id("c".to_string()));
        model.edges.push(Edge::new().id("a->b".to_string()).source_vertex_id("a".to_string()).target_vertex_id("a".to_string()));
        model.edges.push(Edge::new().id("b->c".to_string()).source_vertex_id("b".to_string()).target_vertex_id("c".to_string()));
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
        let a = model.get_vertex("a".to_string()).unwrap();
        assert_eq!(a.id, "a");

        let b = model.get_vertex("x".to_string());
        assert_eq!(b.err(), Some("Vertex with id 'x', is not found.".to_string()));
    }

    #[test]
    fn get_edge_test() {
        let model = create_model();
        let a = model.get_edge("b->c".to_string()).unwrap();
        assert_eq!(a.id, "b->c");

        let b = model.get_edge("x".to_string());
        assert_eq!(b.err(), Some("Edge with id 'x', is not found.".to_string()));
    }
}