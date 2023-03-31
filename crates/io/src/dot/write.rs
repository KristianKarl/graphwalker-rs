use graph::Models;

pub fn write(models: Models) {
    for i in models.models {
        let model = i.1;
        println!("digraph {} {{", model.name.clone().unwrap());

        for j in model.edges {
            let edge = j.1;
            print!(
                "  {} -> {} [label=\"{}",
                model
                    .vertices
                    .get(&edge.source_vertex_id.clone().unwrap())
                    .unwrap()
                    .name
                    .clone()
                    .unwrap(),
                model
                    .vertices
                    .get(&edge.target_vertex_id.clone().unwrap())
                    .unwrap()
                    .name
                    .clone()
                    .unwrap(),
                edge.name.clone().unwrap()
            );
            if edge.guard.is_some() {
                print!("\\n[{}]", edge.guard.clone().unwrap());
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
