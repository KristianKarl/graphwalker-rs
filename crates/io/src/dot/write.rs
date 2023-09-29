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
            print!(
                "  {} [label=\"{}\\nid: {}\"]\n",
                v.id.clone().expect("An id for the vertex."),
                v.name.clone().expect("A name for the vertex."),
                v.id.clone().expect("An id for the vertex.")
            );
        }

        print!("\n");

        for j in &model.edges {
            let edge = j.1;
            print!(
                "  {} -> {} [label=\"{}\\nid: {}",
                &model
                    .vertices
                    .get(
                        &edge
                            .source_vertex_id
                            .clone()
                            .expect("Expexted source vertex id")
                    )
                    .expect("Expexted a source vertex")
                    .id
                    .clone()
                    .expect("Expexted source vertex name"),
                &model
                    .vertices
                    .get(
                        &edge
                            .target_vertex_id
                            .clone()
                            .expect("Expexted a target vertex id")
                    )
                    .expect("Expexted a target vertex")
                    .id
                    .clone()
                    .expect("Expexted a target vertex name"),
                edge.name.clone().expect("Expexted an edge name"),
                edge.id.clone().expect("Expexted an edge id")
            );
            if edge.guard.is_some() {
                print!(
                    "\\nGuard: {}",
                    edge.guard.clone().expect("Expexted a guard for an edge")
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
