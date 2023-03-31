use graph::Models;

pub fn write(models: Models) {
    for i in models.models {
        let model = i.1;
        println!(
            "digraph {} {{",
            model.name.clone().expect("Expected a model name")
        );

        for j in model.edges {
            let edge = j.1;
            print!(
                "  {} -> {} [label=\"{}",
                model
                    .vertices
                    .get(
                        &edge
                            .source_vertex_id
                            .clone()
                            .expect("Expexted source vertex id")
                    )
                    .expect("Expexted a source vertex")
                    .name
                    .clone()
                    .expect("Expexted source vertex name"),
                model
                    .vertices
                    .get(
                        &edge
                            .target_vertex_id
                            .clone()
                            .expect("Expexted a target vertex id")
                    )
                    .expect("Expexted a target vertex")
                    .name
                    .clone()
                    .expect("Expexted a target vertex name"),
                edge.name.clone().expect("Expexted an edge name")
            );
            if edge.guard.is_some() {
                print!(
                    "\\n[{}]",
                    edge.guard.clone().expect("Expexted a guard for an edge")
                );
            }
            if !edge.actions.is_empty() {
                for action in &edge.actions {
                    print!("\\n{action}");
                }
            }
            println!("\"]");
        }
        println!("}}");
    }
}
