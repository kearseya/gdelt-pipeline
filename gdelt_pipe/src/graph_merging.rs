use petgraph::graph::{NodeIndex, UnGraph};
use petgraph::visit::EdgeRef;
// use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
// use std::proccess;
use std::io::{Write};
use rust_stemmers::{Algorithm, Stemmer};
use rayon::prelude::*;

use crate::node_graphing;
use crate::node_graphing::WordNode;


pub fn normalize_word(word: &str) -> String {
    let stemmer = Stemmer::create(Algorithm::English);
    stemmer.stem(&word.to_lowercase()).to_string()
}

pub fn suggest_merges(graph: &UnGraph<WordNode, i32>) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<NodeIndex>> = HashMap::new();

    // Group by stem
    for idx in graph.node_indices() {
        let stem = normalize_word(&graph[idx].word);
        groups.entry(stem).or_default().push(idx);
    }

    let mut suggestions = HashMap::new();

    for nodes in groups.values() {
        if nodes.len() < 2 {
            continue;
        }

        // Choose the highest-count word as the main word
        let main = nodes
            .iter()
            .max_by_key(|idx| graph[**idx].count)
            .unwrap();

        let target = graph[*main].word.clone();

        let variants = nodes
            .iter()
            .filter(|&&idx| idx != *main)
            .map(|&idx| graph[idx].word.clone())
            .collect::<Vec<_>>();

        if !variants.is_empty() {
            suggestions.insert(target,variants);
        }
    }
    suggestions
}

pub fn write_merge_suggestions(
    suggestions: &HashMap<String, Vec<String>>,
    path: &str
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(path)?;

    for (target, variants) in suggestions {
        writeln!(
            file,
            "{} = {}",
            target,
            variants.join(", ")
        )?;
    }
    Ok(())
}

pub fn read_merge_file(path: &str) -> Result<HashMap<String, Vec<String>>, Box<dyn Error>> {
    let contents = std::fs::read_to_string(path)?;
    let mut result = HashMap::new();

    for line in contents.lines() {
        let Some((target, variants)) =
            line.split_once('=')
        else {
            continue;
        };

        let words = variants
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        result.insert(target.trim().to_string(), words);
    }
    Ok(result)
}

pub fn merge_individual_graphs(graph_paths: &[std::path::PathBuf]) -> Result<UnGraph<WordNode, i32>, Box<dyn Error>> {
    let mut merged: UnGraph<WordNode, i32> = UnGraph::new_undirected();
    let mut node_index: HashMap<String, NodeIndex> = HashMap::new();
    let mut pair_weights: HashMap<(String, String), i32> = HashMap::new();

    println!("Merging {} graph files:", graph_paths.len());
    for path in graph_paths {
        println!(" {}", path.display());
        let file_graph = node_graphing::load_graph(path.to_str().unwrap())?;

        for idx in file_graph.node_indices() {
            let word_node = &file_graph[idx];
            let merged_idx = *node_index.entry(word_node.word.clone()).or_insert_with(|| {
                merged.add_node(WordNode { word: word_node.word.clone(), count: 0 })
            });
            merged[merged_idx].count += word_node.count;
        }

        for edge in file_graph.edge_references() {
            let wa = file_graph[edge.source()].word.clone();
            let wb = file_graph[edge.target()].word.clone();
            let (wa, wb) = if wa < wb { (wa, wb) } else { (wb, wa) };
            *pair_weights.entry((wa, wb)).or_insert(0) += *edge.weight();
        }
        // `file_graph` drops here each iteration - never more than one
        // per-file graph resident in memory during the merge.
    }

    for ((wa, wb), weight) in pair_weights {
        let a = node_index[&wa];
        let b = node_index[&wb];
        merged.add_edge(a, b, weight);
    }

    println!("merged graph before returning: {} nodes, {} edges", merged.node_count(), merged.edge_count());

    for path in graph_paths {
        let _ = std::fs::remove_file(path);
    }

    Ok(merged)
}

pub fn merge_graph(
    graph: &UnGraph<WordNode, i32>,
    merges: HashMap<String, Vec<String>>,
) -> UnGraph<WordNode, i32> {

    println!("Building replacement map...");

    // Old node -> replacement word
    let mut replacement: HashMap<NodeIndex, String> = HashMap::new();

    for idx in graph.node_indices() {
        let word = &graph[idx].word;

        for (target, variants) in &merges {
            if word == target || variants.contains(word) {
                replacement.insert(idx, target.clone());
                break;
            }
        }
    }

    println!("Building new nodes...");

    let mut new_graph = UnGraph::new_undirected();

    // Old node -> new node
    let mut node_map: HashMap<NodeIndex, NodeIndex> = HashMap::new();

    // Replacement word -> new node
    let mut word_map: HashMap<String, NodeIndex> = HashMap::new();

    for idx in graph.node_indices() {

        let word = replacement
            .get(&idx)
            .unwrap_or(&graph[idx].word);

        let new_idx =
            *word_map
                .entry(word.clone())
                .or_insert_with(|| {
                    new_graph.add_node(
                        WordNode {
                            word: word.clone(),
                            count: 0,
                        }
                    )
                });

        new_graph[new_idx].count += graph[idx].count;
        node_map.insert(idx, new_idx);
    }

    println!("Combining edges...");

    let edges: Vec<_> = graph.edge_references().collect();

    let edge_weights: HashMap<(NodeIndex, NodeIndex), i32> =
        edges
            .par_iter()
            .fold(
                HashMap::new,
                |mut local, edge| {
                    let mut a = node_map[&edge.source()];
                    let mut b = node_map[&edge.target()];
                    // Remove self-loops created by merging.
                    if a == b {
                        return local;
                    }
                    // Canonical ordering for undirected graph.
                    if a > b {
                        std::mem::swap(&mut a, &mut b);
                    }
                    *local
                        .entry((a, b))
                        .or_insert(0) += *edge.weight();

                    local
                },
            )
            .reduce(
                HashMap::new,
                |mut left, right| {
                    for (edge, weight) in right {
                        *left
                            .entry(edge)
                            .or_insert(0) += weight;
                    }
                    left
                },
            );

    println!("Adding {} merged edges...", edge_weights.len());

    for ((a, b), weight) in edge_weights {
        new_graph.add_edge(a, b, weight);
    }

    println!("Finished: {} nodes, {} edges", new_graph.node_count(), new_graph.edge_count());

    new_graph
}

pub fn merge_graph_input(
    graph: Option<UnGraph<WordNode, i32>>,
    graph_path: &str,
    merge_path: &str,
    save_path: &str,
    suggest: bool,
) -> Result<(), Box<dyn std::error::Error>> {

    println!("Loading graph from {}", graph_path);

    let graph = if !graph_path.is_empty() {
        node_graphing::load_graph(graph_path)?
    } else {
        graph.expect("No graph supplied")
    };

    if suggest {
        let suggestions = suggest_merges(&graph);
        write_merge_suggestions(&suggestions, save_path)?;
        return Ok(());
    }

    println!("Reading merge file from {}", merge_path);

    let merges = read_merge_file(merge_path)?;
    let new_graph = merge_graph(&graph, merges);

    println!("Saving graph to {}", save_path);

    node_graphing::save_graph(&new_graph, save_path)?;

    Ok(())
}
