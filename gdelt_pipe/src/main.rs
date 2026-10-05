use clap::{Parser, Subcommand, ArgAction};
// use petgraph::graph::{NodeIndex, UnGraph};
// use petgraph::visit::EdgeRef;
// use plotters::prelude::*;
// use serde::Deserialize;
// use std::collections::HashMap;
// use std::error::Error;
// use std::f64::consts::PI;

mod csv_parsing;
mod ranking;
mod node_graphing;
mod graph_plotting;
mod graph_merging;
mod compare_graphs;
mod charting;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[allow(non_camel_case_types)]
#[derive(Subcommand)]
enum Commands {
    /// Get the top n most common words
    rank {
        #[arg(short, long)]
        input: String,
        #[arg(short, long, default_value="")]
        include: Option<String>,
        #[arg(short, long, default_value="assets/stop_words.txt")]
        exclude: Option<String>,
        #[arg(short, long, default_value_t=50)]
        n: i32,
        #[arg(short, long, default_value="")]
        domains: Option<String>,
        #[arg(long, action)]
        include_domains: bool,
        #[arg(long, default_value_t = 25)]
        filter_column: usize,
        #[arg(long, default_value = "1")]
        filter_value: String,
        #[arg(long, action=ArgAction::SetFalse)]
        dedup: bool,
        #[arg(short, long, action)]
        verbose: bool, // print end csv
    },
    node {
        #[arg(short, long)]
        input: String,
        #[arg(short, long, default_value="")]
        include: Option<String>,
        #[arg(short, long, default_value="assets/stop_words.txt")]
        exclude: Option<String>,
        #[arg(short, long, default_value="")]
        domains: Option<String>,
        #[arg(long, action)]
        include_domains: bool,
        #[arg(long, default_value_t = 25)]
        filter_column: usize,
        #[arg(long, default_value = "1")]
        filter_value: String,
        #[arg(long, action=ArgAction::SetFalse)]
        dedup: bool,
        #[arg(short, long, default_value="")]
        save: Option<String>,
        #[arg(long, action)]
        low_mem: bool,

    },
    merge {
        #[arg(short, long)]
        graph: Option<String>,
        #[arg(short, long)]
        merge_file: Option<String>,
        #[arg(short, long)]
        save: Option<String>,
        #[arg(short, long, action)]
        suggest: bool,
    },
    compare {
        #[arg(long)]
        graph1: String,
        #[arg(long)]
        graph2: String,
        #[arg(short, long)]
        save: String,
    },
    plot {
        #[arg(short, long)]
        input: String,
        #[arg(short, long, default_value="10")]
        min_count: i32,
        #[arg(short, long, default_value="10")]
        min_weight: i32,
        #[arg(short, long, default_value="10000")]
        spacing: f32,
        #[arg(short, long, default_value="0.7")]
        size_exponant: f32,
        #[arg(short, long, action)]
        diff: bool,
        #[arg(short, long, default_value="")]
        filter: Option<String>,
    },
    chart {
        #[arg(short, long, value_delimiter = ' ', num_args = 2..)]
        graphs: Vec<String>,
    }
}

fn main() {
    let args = Cli::parse();

    match &args.command {
        Some(Commands::rank { input, include, exclude, n, domains, include_domains, filter_column, filter_value, dedup, verbose }) => {
            println!("ranking: {:?}", input);
            let include_path = include.clone().unwrap_or_default();
            let exclude_path = exclude.clone().unwrap_or_default();
            let domains_path = domains.clone().unwrap_or_default();
            ranking::show_top_words(input, &include_path, &exclude_path, n, &domains_path, *include_domains, *filter_column, &filter_value, *dedup, *verbose);
        }
        Some(Commands::node { input, include, exclude, domains, include_domains, filter_column, filter_value, dedup, save, low_mem }) => {
            println!("creating nodes: {}", input);
            let include_path = include.clone().unwrap_or_default();
            let exclude_path = exclude.clone().unwrap_or_default();
            let domains_path = domains.clone().unwrap_or_default();
            let save_path = save.clone().unwrap_or_default();
            match node_graphing::graph_words(input, &include_path, &exclude_path, &domains_path, *include_domains, *filter_column, &filter_value, *dedup, &save_path, *low_mem) {
                Ok(()) => println!("Done"),
                Err(e) => println!("Error: {}", e),
            }
        }
        Some(Commands::merge { graph, merge_file, save, suggest }) => {
            let graph_path = graph.clone().unwrap_or_default();
            let merge_path = merge_file.clone().unwrap_or_default();
            let save_path = save.clone().unwrap_or_default();
            match graph_merging::merge_graph_input(None, &graph_path, &merge_path, &save_path, *suggest) {
                Ok(()) => println!("Done"),
                Err(e) => println!("Error: {}", e),
            }
        }
        Some(Commands::compare { graph1, graph2, save }) => {
            match compare_graphs::create_normalised_diff(&graph1, &graph2, &save) {
                Ok(()) => println!("Done"),
                Err(e) => println!("Error: {}", e),
            }
        }
        Some(Commands::plot { input, min_count, min_weight, spacing, size_exponant, diff, filter }) => {
            let filter_path = filter.clone().unwrap_or_default();
            println!("plotting nodes: {}",  input);
            let _ = graph_plotting::plot_graph(&input, &min_count, &min_weight, &spacing, &size_exponant, *diff, &filter_path);
        }
        Some(Commands::chart { graphs }) => {
            let _ = charting::plot_chart(graphs);
        }
        None => {
            println!("There was no subcommand given");
        }
    }
}
