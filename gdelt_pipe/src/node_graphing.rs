use petgraph::graph::{NodeIndex, UnGraph};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufReader, BufWriter};
// use ext_sort::{buffer::mem::MemoryLimitedBufferBuilder, ExternalSorter, ExternalSorterBuilder};

use crate::csv_parsing;
use crate::graph_merging;

#[derive(Serialize, Deserialize, Clone)]
pub struct WordNode {
    pub word: String,
    pub count: i32,
}

// Create graph
pub fn build_word_graph(url_words: Vec<Vec<String>>) -> Result<UnGraph<WordNode, i32>, Box<dyn Error>> { //&[Vec<String>
    // Count word occurrences
    let mut counts: HashMap<String, i32> = HashMap::new();
    for row in &url_words {
        for word in row {
            *counts.entry(word.clone()).or_insert(0) += 1;
        }
    }

    println!("Counting...");
    let mut pair_counts: HashMap<(String, String), i32> = HashMap::new();
    for row in url_words {
        let unique_words: HashSet<&String> = row.iter().collect();
        let unique_words: Vec<&String> = unique_words.into_iter().collect();

        for i in 0..unique_words.len() {
            for j in (i + 1)..unique_words.len() {
                let (a, b) = if unique_words[i] < unique_words[j] {
                    (unique_words[i].clone(), unique_words[j].clone())
                } else {
                    (unique_words[j].clone(), unique_words[i].clone())
                };
                *pair_counts.entry((a, b)).or_insert(0) += 1;
            }
        }
    }

    println!("Creating edges...");
    // Build the graph from the finished counts
    let mut graph: UnGraph<WordNode, i32> = UnGraph::new_undirected();
    let mut node_index: HashMap<String, NodeIndex> = HashMap::new();

    for (word, count) in counts {
        let idx = graph.add_node(WordNode { word: word.clone(), count });
        node_index.insert(word, idx);
    }

    for ((word_a, word_b), weight) in pair_counts {
        let a = node_index[&word_a];
        let b = node_index[&word_b];
        graph.add_edge(a, b, weight);
    }

    Ok(graph)
}


// For cytoscape

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn export_graph_graphml(
    graph: &UnGraph<WordNode, i32>,
    path: &str,
    min_count: i32,
) -> Result<(), Box<dyn Error>> {
    let kept_nodes: HashSet<NodeIndex> = graph
        .node_indices()
        .filter(|&idx| graph[idx].count > min_count)
        .collect();

    let mut xml = String::new();
    writeln!(xml, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(xml, r#"<graphml xmlns="http://graphml.graphdrawing.org/xmlns">"#)?;
    writeln!(xml, r#"  <key id="label" for="node" attr.name="label" attr.type="string"/>"#)?;
    writeln!(xml, r#"  <key id="count" for="node" attr.name="count" attr.type="int"/>"#)?;
    writeln!(xml, r#"  <key id="weight" for="edge" attr.name="weight" attr.type="int"/>"#)?;
    writeln!(xml, r#"  <graph id="G" edgedefault="undirected">"#)?;

    let mut node_ids: HashMap<NodeIndex, String> = HashMap::new();
    for (i, &idx) in kept_nodes.iter().enumerate() {
        let node_id = format!("n{i}");
        node_ids.insert(idx, node_id.clone());

        let word = &graph[idx];
        writeln!(xml, r#"    <node id="{node_id}">"#)?;
        writeln!(xml, r#"      <data key="label">{}</data>"#, escape_xml(&word.word))?;
        writeln!(xml, r#"      <data key="count">{}</data>"#, word.count)?;
        writeln!(xml, "    </node>")?;
    }

    let mut edge_count = 0;
    for edge in graph.edge_references() {
        let (Some(src), Some(dst)) = (node_ids.get(&edge.source()), node_ids.get(&edge.target()))
        else {
            continue;
        };
        writeln!(xml, r#"    <edge source="{src}" target="{dst}">"#)?;
        writeln!(xml, r#"      <data key="weight">{}</data>"#, edge.weight())?;
        writeln!(xml, "    </edge>")?;
        edge_count += 1;
    }

    writeln!(xml, "  </graph>")?;
    writeln!(xml, "</graphml>")?;

    println!("Exporting {} nodes, {} edges to {} (GraphML)", kept_nodes.len(), edge_count, path);
    std::fs::write(path, xml)?;
    Ok(())
}

// Filtering

pub fn remove_words_from_graph(
    graph: &mut UnGraph<WordNode, i32>,
    words_to_remove: HashSet<String>) {
    println!("Filtering words from graph...");

    let nodes_to_remove: Vec<NodeIndex> = graph
        .node_indices()
        .filter(|&idx| words_to_remove.contains(graph[idx].word.as_str()))
        .collect();

    for idx in nodes_to_remove {
        graph.remove_node(idx);
    }
}


// Save and load

pub fn save_graph(graph: &UnGraph<WordNode, i32>, path: &str) -> Result<(), Box<dyn Error>> {
    if path.ends_with(".bin") {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, graph)?;
    } else if path.ends_with(".json") {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, graph)?;
    } else if path.ends_with(".graphml") {
        // GraphML is for external tools (Cytoscape, Gephi)
        export_graph_graphml(graph, path, 0)?;
    } else {
        println!("Save path must be .bin, .json, or .graphml");
    }
    Ok(())
}

pub fn load_graph(path: &str) -> Result<UnGraph<WordNode, i32>, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    if path.ends_with(".bin") {
        let graph: UnGraph<WordNode, i32> = bincode::deserialize_from(reader)?;
        Ok(graph)
    } else if path.ends_with(".json") {
        let graph: UnGraph<WordNode, i32> = serde_json::from_reader(reader)?;
        Ok(graph)
    } else if path.ends_with(".graphml") {
        Err("Cannot load .graphml files — GraphML is export-only (for Cytoscape/Gephi). Use .bin or .json to save/reload.".into())
    } else {
        Err("Load path must be .bin or .json".into())
    }
}


// Create graph
pub fn graph_words(
    input: &str,
    include: &str,
    exclude: &str,
    domains: &str,
    include_domains: bool,
    filter_column: usize,
    filter_value: &str,
    dedup: bool,
    save_path: &str,
    low_mem: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let domain_set = if domains.is_empty() { HashSet::new() } else { csv_parsing::load_word_set(domains) };
    let include_set = if include.is_empty() { HashSet::new() } else { csv_parsing::load_word_set(include) };
    let exclude_set = if exclude.is_empty() { HashSet::new() } else { csv_parsing::load_word_set(exclude) };

    let analyser = csv_parsing::UrlAnalyser::new(
        domain_set,
        include_set,
        exclude_set,
        include_domains, 
        filter_column,
        filter_value.to_string(),
        dedup,
    );

    // let mut graph: UnGraph<WordNode, i32> = UnGraph::<WordNode, i32>::new_undirected();

    let graph: UnGraph<WordNode, i32> = if !low_mem {
        let url_words = analyser.analyse(input);

        println!("Total lines parsed: {}", analyser.lines_parsed());
        println!("Number of URLs analysed: {}", url_words.len());
        
        let graph = build_word_graph(url_words)?;
        println!("Graph: {} nodes, {} edges", graph.node_count(), graph.edge_count());
        graph
    } else {
        let save_path_obj = std::path::Path::new(save_path);
        let parent_dir = save_path_obj.parent().unwrap_or_else(|| std::path::Path::new("."));
        let output_dir = parent_dir.join("per_file_graphs");

        let graph_paths = analyser.process_directory_to_graphs(input, &output_dir)?;
        println!("Total lines parsed: {}", analyser.lines_parsed());
        println!("Number of URLs analysed: {}", analyser.lines_used());
        let graph = graph_merging::merge_individual_graphs(&graph_paths)?;
        println!("Graph: {} nodes, {} edges", graph.node_count(), graph.edge_count());
        println!("Removing individual graphs");
        let _ = std::fs::remove_dir_all(&output_dir); 
        graph
    };

    if !save_path.is_empty() {
        let _ = save_graph(&graph, &save_path);
        println!("Saved graph: {}", save_path);
    }

    Ok(())
}
