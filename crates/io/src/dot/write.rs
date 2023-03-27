use graph::Models;

pub fn write(models: Models) {
    for model in models.models {
        println!("digraph {} {{", model.name);

        for edge in model.edges.iter() {
            print!(
                "  {} -> {} [label=\"{}",
                model
                    .get_vertex(edge.source_vertex_id.clone())
                    .unwrap()
                    .name,
                model
                    .get_vertex(edge.target_vertex_id.clone())
                    .unwrap()
                    .name,
                edge.name
            );
            if !edge.guard.is_empty() {
                print!("\\n[{}]", edge.guard);
            }
            if !edge.actions.is_empty() {
                for action in edge.actions.iter() {
                    print!("\\n{}", action);
                }
            }
            println!("\"]");
        }
        println!("}}");
    }
}
