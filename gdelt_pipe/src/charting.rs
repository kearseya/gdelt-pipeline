use eframe::egui::{self};
use egui_plot::{Line, Plot, PlotPoints};
use petgraph::graph::{NodeIndex, UnGraph};
// use petgraph::visit::EdgeRef;
use std::collections::{HashMap};
// use std::path::Path;
// use std::ffi::OsStr;

use crate::node_graphing;
use crate::node_graphing::{WordNode};
// use crate::csv_parsing::load_word_set;
use crate::compare_graphs;

struct Series {
    word: String,
    points: Vec<[f64; 2]>,
}

struct ChartingApp {
    graphs: HashMap<String, UnGraph<WordNode, i32>>,
    normalised_graphs: HashMap<String, UnGraph<WordNode, i32>>,
    graph_totals: HashMap<String, f64>,

    search: String,
    words: Vec<String>,
    normalised: bool,

    chart: Vec<Series>,
}

impl ChartingApp {
    fn new(graphs: HashMap<String, UnGraph<WordNode, i32>>) -> Self {
        let mut graph_totals: HashMap<String, f64> = HashMap::<String, f64>::new();
        let mut normalised_graphs: HashMap<String, UnGraph<WordNode, i32>> = HashMap::<String, UnGraph<WordNode, i32>>::new();
        for (time, graph) in graphs.iter() {
            // let time_path: &Path = Path::new(time);
            // let time_str: String = time_path.file_name().and_then(OsStr::to_str).unwrap().to_string();
            let total: f64 = graph.node_weights().map(|n| n.count as f64).sum();
            graph_totals.insert(time.clone(), total);
            let mut norm = graph.clone();
            compare_graphs::normalise_graph(&mut norm);
            normalised_graphs.insert(time.clone(), norm);
        }

        ChartingApp {
            graphs,
            normalised_graphs,
            graph_totals,
            search: "".to_string(),
            words: Vec::<String>::new(),
            normalised: false,
            chart: Vec::<Series>::new(),
        }
    }

    fn rebuild_chart(&mut self) {
        self.chart.clear();

        let mut times: Vec<_> = self.graphs.keys().collect();
        times.sort();

        for word in &self.words {
            let is_or_expr = word.contains('|');
            let mut points = Vec::new();

            for (i, time) in times.iter().enumerate() {
                
                // let Some(x) = time.parse::<f64>().ok() else {
                //     eprintln!("Warning: could not parse time '{}' as a number, skipping", time);
                //     continue;
                // };
                
                let value = if self.normalised && is_or_expr {
                    // OR under normalisation: evaluate against the RAW graph
                    // and divide by that year's true total (graph_totals),
                    // rather than relying on the precomputed normalised
                    // graph's fixed divisor - this correctly accounts for
                    // totals varying year to year instead of assuming a
                    // constant scale.
                    let raw_graph = &self.graphs[*time];
                    evaluate_word_expression(raw_graph, word).map(|raw| {
                        let total = self.graph_totals[*time] as f64;
                        if total > 0.0 { (raw as f64 / total) * 100.0 } else { 0.0 }
                    })
                } else if self.normalised {
                    // Plain word / pure AND under normalisation: unchanged -
                    // still uses the precomputed normalised graph.
                    let graph = &self.normalised_graphs[*time];
                    evaluate_word_expression(graph, word).map(|raw| raw as f64 / 10_000_000.0)
                } else {
                    let graph = &self.graphs[*time];
                    evaluate_word_expression(graph, word).map(|raw| raw as f64)
                };

                if let Some(v) = value {
                    points.push([i as f64, v]); //i as f64
                }
            }

            self.chart.push(Series {
                word: word.clone(),
                points,
            });
        }
    }

    fn add_word_to_chart(&mut self, word: String) {
        if self.words.contains(&word) {
            return;
        }

        self.words.push(word);
        self.rebuild_chart();
    }

    fn remove_word_from_chart(&mut self, word: &str) {
        self.words.retain(|w| w != word);
        self.rebuild_chart();
    }
}

impl eframe::App for ChartingApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("search_bar")
            .frame(egui::Frame::default().fill(egui::Color32::from_rgb(20,20,20)))
            .show(ui, |ui: &mut egui::Ui| {
            ui.horizontal(|ui| {
                ui.label("Search:");
                let search_response = ui.text_edit_singleline(&mut self.search);
                if search_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                {
                    let word = self.search.trim().to_lowercase();

                    if !word.is_empty() {
                        self.add_word_to_chart(word);
                    }

                    self.search.clear();
                }
                if ui.checkbox(&mut self.normalised, "Normalised").changed() {
                    self.rebuild_chart();
                }
            });
            ui.horizontal_wrapped(|ui| {
                let mut remove = None;
                for word in &self.words {
                    if ui.button(format!("{} ✕", word)).clicked() {
                        remove = Some(word.clone());
                    }
                }
                if let Some(word) = remove {
                    self.remove_word_from_chart(&word);
                }
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let plot = Plot::new("graph_plot")
                .view_aspect(2.0)
                .legend(egui_plot::Legend::default());

            plot.show(ui, |plot_ui| {
                for series in &self.chart {
                    let points = PlotPoints::from(series.points.clone());
                    plot_ui.line(Line::new(series.word.clone(), points));
                } 
            });
        });
    } 
}


pub fn plot_chart(graph_paths: &Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut graphs: HashMap<String, UnGraph<WordNode, i32>> = HashMap::<String, UnGraph<WordNode, i32>>::new();
    for path in graph_paths {
        let time: String = path.split('_').next().unwrap_or_default().to_string();
        let graph: UnGraph<WordNode, i32> = node_graphing::load_graph(&path)?;
        graphs.insert(time, graph);
    }
    
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };
    let result = eframe::run_native(
        "Word Co-occurrence Graph Viewer",
        native_options,
        Box::new(move |_cc| {
            Ok(Box::new(ChartingApp::new(graphs)))
        }),
    );
    // println!("eframe exited: {:?}", result);
    result.map_err(|e| format!("eframe error: {e}").into())
}


// --- AST ---
#[derive(Debug, Clone)]
enum Expr {
    Word(String),
    And(Box<Expr>, Box<Expr>), // &
    Or(Box<Expr>, Box<Expr>), // |
    Cross(Box<Expr>, Box<Expr>), // +
}

// --- Tokenizer ---
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    And,
    Or,
    Cross,
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current_word = String::new();

    let flush = |current_word: &mut String, tokens: &mut Vec<Token>| {
        let trimmed = current_word.trim();
        if !trimmed.is_empty() {
            tokens.push(Token::Word(trimmed.to_string()));
        }
        current_word.clear();
    };

    for c in input.chars() {
        match c {
            '&' => { flush(&mut current_word, &mut tokens); tokens.push(Token::And); }
            '|' => { flush(&mut current_word, &mut tokens); tokens.push(Token::Or); }
            '(' => { flush(&mut current_word, &mut tokens); tokens.push(Token::LParen); }
            ')' => { flush(&mut current_word, &mut tokens); tokens.push(Token::RParen); }
            '+' => { flush(&mut current_word, &mut tokens); tokens.push(Token::Cross); }
            _ => current_word.push(c),
        }
    }
    flush(&mut current_word, &mut tokens);
    tokens
}

// --- Recursive-descent parser ---
// Grammar: expr := and_expr ('|' and_expr)*
//          and_expr := atom ('&' atom)*
//          atom := '(' expr ')' | WORD
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
    fn advance(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn parse_expr(&mut self) -> Option<Expr> {
        let mut left = self.parse_cross_expr()?;
        while self.peek() == Some(&Token::Or) {
            self.advance();
            let right = self.parse_cross_expr()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Some(left)
    }

    fn parse_cross_expr(&mut self) -> Option<Expr> {
        let mut left = self.parse_and_expr()?;
        while self.peek() == Some(&Token::Cross) {
            self.advance();
            let right = self.parse_and_expr()?;
            left = Expr::Cross(
                Box::new(left),
                Box::new(right),
            );
        }

        Some(left)
    }

    fn parse_and_expr(&mut self) -> Option<Expr> {
        let mut left = self.parse_atom()?;
        while self.peek() == Some(&Token::And) {
            self.advance();
            let right = self.parse_atom()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Some(left)
    }

    fn parse_atom(&mut self) -> Option<Expr> {
        match self.advance()? {
            Token::LParen => {
                let inner = self.parse_expr()?;
                if self.peek() == Some(&Token::RParen) {
                    self.advance();
                }
                Some(inner)
            }
            Token::Word(w) => Some(Expr::Word(w)),
            _ => None,
        }
    }
}

// cross
fn collect_alternatives(expr: &Expr) -> Vec<Expr> {
    match expr {
        Expr::Or(a, b) => {
            let mut v = collect_alternatives(a);
            v.extend(collect_alternatives(b));
            v
        }

        _ => vec![expr.clone()],
    }
}

fn expand_cross(expr: Expr) -> Expr {
    match expr {

        Expr::Word(_) => expr,

        Expr::And(a, b) => {
            Expr::And(Box::new(expand_cross(*a)), Box::new(expand_cross(*b)))
        }

        Expr::Or(a, b) => {
            Expr::Or(Box::new(expand_cross(*a)), Box::new(expand_cross(*b)))
        }

        Expr::Cross(a, b) => {
            let left = collect_alternatives(&expand_cross(*a));
            let right = collect_alternatives(&expand_cross(*b));
            let mut terms = Vec::new();
            for l in &left {
                for r in &right {
                    terms.push(Expr::And(Box::new(l.clone()), Box::new(r.clone())));
                }
            }
            let mut iter = terms.into_iter();
            let mut expr = iter.next().unwrap();
            for term in iter {
                expr = Expr::Or(Box::new(expr), Box::new(term));
            }
            expr
        }
    }
}

fn parse_expression(input: &str) -> Option<Expr> {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return None;
    }
    Parser::new(tokens).parse_expr()
}

// --- Distribute AND over OR to get an OR-of-flat-AND-clauses ---
// e.g. "cake & (recipe | recipies)" -> [[cake, recipe], [cake, recipies]]
fn to_dnf(expr: &Expr) -> Vec<Vec<String>> {
    match expr {
        Expr::Word(w) => vec![vec![w.clone()]],
        Expr::Or(a, b) => {
            let mut clauses = to_dnf(a);
            clauses.extend(to_dnf(b));
            clauses
        }
        Expr::And(a, b) => {
            let clauses_a = to_dnf(a);
            let clauses_b = to_dnf(b);
            let mut result = Vec::new();
            for ca in &clauses_a {
                for cb in &clauses_b {
                    let mut merged = ca.clone();
                    merged.extend(cb.clone());
                    result.push(merged);
                }
            }
            result
        }
        Expr::Cross(_, _) => {
            panic!("Cross expression reached to_dnf(); expand_cross() should have removed it");
        }
    }
}

// --- Evaluate one flat AND-clause: pairwise-minimum edge weight, or
// plain node count if it's a single word. Same logic as before. ---
fn evaluate_clause(graph: &UnGraph<WordNode, i32>, terms: &[String]) -> Option<i32> {
    let node_indices: Vec<Option<NodeIndex>> = terms
        .iter()
        .map(|t| graph.node_indices().find(|&n| &graph[n].word == t))
        .collect();

    if node_indices.iter().all(|n| n.is_none()) {
        return None;
    }

    if terms.len() == 1 {
        return Some(node_indices[0].map(|idx| graph[idx].count).unwrap_or(0));
    }

    let mut min_weight: Option<i32> = None;
    for i in 0..terms.len() {
        for j in (i + 1)..terms.len() {
            let w = match (node_indices[i], node_indices[j]) {
                (Some(a), Some(b)) => graph.find_edge(a, b).map(|e| graph[e]).unwrap_or(0),
                _ => 0,
            };
            min_weight = Some(min_weight.map_or(w, |c| c.min(w)));
        }
    }
    min_weight
}

/// Parses `expr_str` (supporting &, |, and parentheses with proper
/// precedence/distribution), then evaluates it against `graph`. Sums the
/// value of every resulting AND-clause; returns None only if every
/// clause is entirely absent from this year's data.
fn evaluate_word_expression(graph: &UnGraph<WordNode, i32>, expr_str: &str) -> Option<i32> {
    let expr = parse_expression(expr_str)?;
    // println!("{:#?}", parse_expression(expr_str));
    let expr = expand_cross(expr);
    let dnf = to_dnf(&expr);
    // println!("{:#?}", dnf);

    let mut sum = 0i32;
    let mut any_found = false;
    for clause in &dnf {
        if let Some(v) = evaluate_clause(graph, clause) {
            sum += v;
            any_found = true;
        }
    }
    if any_found { Some(sum) } else { None }
}
