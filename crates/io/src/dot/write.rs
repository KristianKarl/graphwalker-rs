use std::sync::Arc;

use graph::Models;

pub fn write(models: Arc<Models>) {
    for i in &*models.models {
        let model = i.1;
        println!(
            "digraph {} {{",
            model.name.clone().expect("Expected a model name")
        );

        for n in &*model.vertices.read().unwrap() {
            let v = n.1;
            println!(
                "  {} [label=\"{}\\nid: {}\"]",
                v.id,
                v.name.clone().expect("A name for the vertex."),
                v.id
            );
        }

        println!();

        for j in &*model.edges.read().unwrap() {
            let edge = j.1;
            print!(
                "  {} -> {} [label=\"{}\\nid: {}",
                &model
                    .vertices
                    .read()
                    .unwrap()
                    .get(&edge.source_vertex_id)
                    .expect("Source vertex")
                    .id,
                &model
                    .vertices
                    .read()
                    .unwrap()
                    .get(&edge.target_vertex_id)
                    .expect("Target vertex")
                    .id,
                edge.name.clone().expect("Edge name"),
                edge.id
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
