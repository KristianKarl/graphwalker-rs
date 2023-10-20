use graph::Models;

pub fn write(models: Models) {
    for i in models.models {
        let model = i.1;
        println!(
            "digraph {} {{",
            model.name.clone().expect("Expected a model name")
        );

        for n in &model.vertices {
            let v = n.1;
            println!(
                "  {} [label=\"{}\\nid: {}\"]",
                v.id.clone().expect("An id for the vertex."),
                v.name.clone().expect("A name for the vertex."),
                v.id.clone().expect("An id for the vertex.")
            );
        }

        println!();

        for j in &model.edges {
            let edge = j.1;
            print!(
                "  {} -> {} [label=\"{}\\nid: {}",
                &model
                    .vertices
                    .get(&edge.source_vertex_id.clone().expect("Source vertex id"))
                    .expect("Source vertex")
                    .id
                    .clone()
                    .expect("Source vertex name"),
                &model
                    .vertices
                    .get(&edge.target_vertex_id.clone().expect("Target vertex id"))
                    .expect("Target vertex")
                    .id
                    .clone()
                    .expect("Target vertex name"),
                edge.name.clone().expect("Edge name"),
                edge.id.clone().expect("Edge id")
            );
            if edge.guard.is_some() {
                print!(
                    "\\nGuard: {}",
                    edge.guard.clone().expect("Guard for an edge")
                );
            }
            if !edge.actions.is_empty() {
                for action in &edge.actions {
                    print!("\\nAction: {action}");
                }
            }
            println!("\"]");
        }
        println!("}}");
    }
}
