use dot_writer::DotWriter;

pub fn write(models: crate::graph::model::Models) {
    for model in models.models {
        let mut output_bytes = Vec::new();
        {
            let mut writer = DotWriter::from(&mut output_bytes);
            let mut digraph = writer.digraph();
            for edge in model.edges {
                digraph.edge(edge.source_vertex_id, edge.target_vertex_id);
            }
        }
        println!("{}", String::from_utf8(output_bytes).unwrap());
    }
}