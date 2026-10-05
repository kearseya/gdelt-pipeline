use std::collections::{HashMap, HashSet};
use petgraph::graph::{NodeIndex, UnGraph};
use petgraph::visit::EdgeRef;

use crate::node_graphing;
use crate::node_graphing::WordNode;

pub fn normalise_graph(graph: &mut UnGraph<WordNode, i32>) {
    let total_count: u64 = graph.node_weights().map(|n| n.count as u64).sum();
    let total_weight: u64 = graph.edge_references().map(|e| *e.weight() as u64).sum();
    println!("Total count: {}, Total weights: {}", total_count, total_weight);
    // Normalise counts
    for node in graph.node_weights_mut() {
        node.count = ((node.count as f64 / total_count as f64) * 1_000_000_000.0).round() as i32;
    }
    // Normalise weights
    for weight in graph.edge_weights_mut() {
        *weight = ((*weight as f64 / total_weight as f64) * 1_000_000_000.0).round() as i32;
    }
}

fn edge_key<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

pub fn diff_graph(
    graph1: &UnGraph<WordNode, i32>, 
    graph2: &UnGraph<WordNode, i32>) 
    -> UnGraph<WordNode, i32> {
    let mut graph2_nodes = HashMap::<String, NodeIndex>::new();
    for idx in graph2.node_indices() {
        graph2_nodes.insert(graph2[idx].word.clone(), idx);
    }

    let mut diff = UnGraph::<WordNode, i32>::new_undirected();

    println!("Creating nodes...");
    // Handle nodes
    let mut diff_nodes = HashMap::<String, NodeIndex>::new();

    for idx in graph1.node_indices() {
        let node = &graph1[idx];

        let count = match graph2_nodes.get(&node.word) {
            Some(&other) => node.count - graph2[other].count,
            None => node.count,
        };

        let new_idx = diff.add_node(WordNode {
            word: node.word.clone(),
            count,
        });

        diff_nodes.insert(node.word.clone(), new_idx);
    }

    for idx in graph2.node_indices() {
        let node = &graph2[idx];

        if !diff_nodes.contains_key(&node.word) {
            let new_idx = diff.add_node(WordNode {
                word: node.word.clone(),
                count: -node.count,
            });

            diff_nodes.insert(node.word.clone(), new_idx);
        }
    }

    println!("Creating edges...");
    // Handle edges
    let mut graph2_edges = HashMap::new();

    for edge in graph2.edge_references() {
        let a = &graph2[edge.source()].word;
        let b = &graph2[edge.target()].word;

        graph2_edges.insert(edge_key(a, b), *edge.weight());
    }

    let mut seen = HashSet::new();

    for edge in graph1.edge_references() {
        let a = &graph1[edge.source()].word;
        let b = &graph1[edge.target()].word;

        let key = edge_key(a, b);

        let weight = *edge.weight()
            - graph2_edges.get(&key).copied().unwrap_or(0);

        diff.add_edge(diff_nodes[a], diff_nodes[b], weight);

        seen.insert(key);
    }

    for edge in graph2.edge_references() {
        let a = &graph2[edge.source()].word;
        let b = &graph2[edge.target()].word;

        let key = edge_key(a, b);

        if seen.contains(&key) {
            continue;
        }

        diff.add_edge(diff_nodes[a], diff_nodes[b], -*edge.weight());
    }

    diff
}

pub fn create_normalised_diff(graph1_path: &str, graph2_path: &str, save_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut graph1: UnGraph<WordNode, i32> = node_graphing::load_graph(graph1_path)?;
    let mut graph2: UnGraph<WordNode, i32> = node_graphing::load_graph(graph2_path)?;

    normalise_graph(&mut graph1);
    normalise_graph(&mut graph2);

    let compared: UnGraph<WordNode, i32> = diff_graph(&graph1, &graph2);
    node_graphing::save_graph(&compared, save_path)?;

    Ok(())
}
