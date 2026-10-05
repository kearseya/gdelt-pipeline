use csv::WriterBuilder;

// Shared with normalise_graph
const NORMALISE_SCALE: f64 = 1_000_000_000.0;

pub fn export_top_node_changes(
    diff: &UnGraph<WordNode, i32>,
    top_n: usize,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut nodes: Vec<&WordNode> = diff.node_weights().collect();
    nodes.sort_by_key(|n| std::cmp::Reverse(n.count.unsigned_abs()));

    let mut writer = WriterBuilder::new().from_path(output_path)?;
    writer.write_record(&["word", "diff_raw", "diff_percent"])?;

    for node in nodes.into_iter().take(top_n) {
        let percent = node.count as f64 / NORMALISE_SCALE * 100.0;
        writer.write_record(&[
            node.word.clone(),
            node.count.to_string(),
            format!("{:.4}", percent),
        ])?;
    }

    writer.flush()?;
    Ok(())
}

pub fn export_top_edge_changes(
    diff: &UnGraph<WordNode, i32>,
    top_n: usize,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut edges: Vec<(String, String, i32)> = diff
        .edge_references()
        .map(|e| {
            let a = diff[e.source()].word.clone();
            let b = diff[e.target()].word.clone();
            (a, b, *e.weight())
        })
        .collect();
    edges.sort_by_key(|(_, _, w)| std::cmp::Reverse(w.unsigned_abs()));

    let mut writer = WriterBuilder::new().from_path(output_path)?;
    writer.write_record(&["word_a", "word_b", "diff_raw", "diff_percent"])?;

    for (a, b, weight) in edges.into_iter().take(top_n) {
        let percent = weight as f64 / NORMALISE_SCALE * 100.0;
        writer.write_record(&[a, b, weight.to_string(), format!("{:.4}", percent)])?;
    }

    writer.flush()?;
    Ok(())
}
