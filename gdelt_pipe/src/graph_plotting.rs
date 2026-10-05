use eframe::egui;
use petgraph::graph::{NodeIndex, UnGraph};
use petgraph::visit::EdgeRef;
// use rayon::prelude::*;
// use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
// use std::error::Error;
// use std::fs::File;
// use std::io::BufReader;
// use std::io::{self, Write};
// use std::time::Instant;

use crate::node_graphing;
use crate::node_graphing::{WordNode, remove_words_from_graph};
use crate::csv_parsing::load_word_set;

fn compute_layout(
    graph: &UnGraph<WordNode, i32>,
    node_ids: &[NodeIndex],
    spacing: f32,
) -> HashMap<NodeIndex, egui::Vec2> {

    let mut order: Vec<NodeIndex> = node_ids.to_vec();
    order.sort_by(|&a, &b| graph[b].count.abs().cmp(&graph[a].count.abs())); // descending

    let golden_angle: f32 = std::f32::consts::PI * (3.0 - 5f32.sqrt());

    order
        .into_iter()
        .enumerate()
        .map(|(i, idx)| {
            let radius = spacing * (i as f32).sqrt();
            let theta = i as f32 * golden_angle;
            (idx, egui::vec2(radius * theta.cos(), radius * theta.sin()))
        })
        .collect()
}


struct ViewerApp {
    graph: UnGraph<WordNode, i32>,
    kept_nodes: Vec<NodeIndex>,
    positions: HashMap<NodeIndex, egui::Vec2>, 
    neighbours: HashSet<NodeIndex>,

    // Camera
    pan: egui::Vec2,
    zoom: f32,

    // Interaction
    dragging_node: Option<NodeIndex>,
    selected_node: Option<NodeIndex>,

    // Rendering
    min_weight: i32,
    max_count: i32,
    max_weight: i32,
    spacing: f32,
    size_exponent: f32,
    hide_edges: bool,

    // Search UI
    search: String,

    // Display options
    dim_unrelated: bool,
    show_edge_panel: bool,
    diff: bool,

    focus_radius: f32,
    golden_angle: f32,
    viewport_size: egui::Vec2,
}

impl ViewerApp {
    fn new(
        graph: UnGraph<WordNode, i32>,
        min_count: i32,
        min_weight: i32,
        spacing: f32,
        size_exponent: f32,
        diff: bool,
    ) -> Self {
        let kept_nodes: Vec<NodeIndex> = graph
            .node_indices()
            .filter(|&idx| graph[idx].count.abs() > min_count)
            .collect();

        let positions = compute_layout(&graph, &kept_nodes, 200.0);

        let max_count = kept_nodes
            .iter()
            .map(|&idx| graph[idx].count)
            .max()
            .unwrap_or(1);
        let max_weight = graph.edge_weights().copied().max().unwrap_or(1);

        ViewerApp {
            graph,
            kept_nodes,
            positions,
            neighbours: HashSet::<NodeIndex>::new(),

            pan: egui::Vec2::ZERO,
            zoom: 1.0,

            dragging_node: None,
            selected_node: None,

            min_weight,
            max_count,
            max_weight,
            spacing,
            size_exponent,
            diff,
            hide_edges: true,

            search: String::new(),

            dim_unrelated: true,
            // auto_zoom_on_search: true,
            show_edge_panel: false,

            focus_radius: 10.0,
            golden_angle: std::f32::consts::PI * (3.0 - 5f32.sqrt()),
            viewport_size: egui::Vec2::new(10.0, 10.0),
        }
    }

    fn world_to_screen(&self, world: egui::Vec2, center: egui::Vec2) -> egui::Pos2 {
        center.to_pos2() + (world + self.pan) * self.zoom
    }

    fn select_node(&mut self, idx: NodeIndex, rearrange: bool) {
        self.selected_node = Some(idx);

        if let Some(pos) = self.positions.get(&idx) {
            self.pan = -*pos;
        }

        let target = self.selected_node;
        let node_set: HashSet<NodeIndex> = self.kept_nodes.iter().copied().collect();

        self.neighbours =
            if let Some(node) = target {
                self.graph
                    .edges(node)
                    .filter(|e| e.weight().abs() >= self.min_weight)
                    .map(|e|
                        if e.source() == node { e.target() } 
                        else { e.source() } )
                    .filter(|n| node_set.contains(n))
                    .collect()
            } else {
                HashSet::new()
            };

        match rearrange {
            true => self.arrange_neighbours_in_circle(idx, self.spacing),
            false => (),
        }

        let margin = 100.0;
        let diameter = self.focus_radius * 2.0 + margin;

        self.zoom = (self.viewport_size.x.min(self.viewport_size.y) / diameter)
            .clamp(0.15, 5.0);

    }

    fn arrange_neighbours_in_circle(&mut self, center: NodeIndex, _spacing: f32) {
        let center_pos = self.positions[&center];

        let mut neighbours: Vec<(NodeIndex, i32)> = self.graph
            .edges(center)
            .filter(|e| e.weight().abs() >= self.min_weight)
            .filter_map(|e| {
                let other = if e.source() == center {
                    e.target()
                } else {
                    e.source()
                };

                self.positions
                    .contains_key(&other)
                    .then_some((other, *e.weight()))
            })
            .collect();

        if neighbours.is_empty() {
            return;
        }

        // Strongest first
        neighbours.sort_by(|a, b| b.1.cmp(&a.1));

        let max_weight = neighbours
            .iter()
            .map(|(_, w)| *w)
            .max()
            .unwrap_or(1) as f32;

        let n = neighbours.len() as f32;

        // Radius scales slowly with number of nodes
        let inner_radius = 80.0;
        let outer_radius = 350.0 + 45.0 * n.sqrt();

        for (i, (node, weight)) in neighbours.iter().enumerate() {
            let rank = i as f32 / (n - 1.0).max(1.0);
            let weight_t = *weight as f32 / max_weight;
            let rank_radius = rank.sqrt();
            let weight_pull = weight_t.powf(0.8) * 0.35;

            let radius_t = (rank_radius * (1.0 - weight_pull))
                .clamp(0.0, 1.0);

            let radius = inner_radius + radius_t * (outer_radius - inner_radius);

            // Fibonacci spiral angle
            let angle = i as f32 * self.golden_angle;
            let target = center_pos + egui::vec2(radius * angle.cos(), radius * angle.sin());

            if let Some(pos) = self.positions.get_mut(node) {
                *pos = target;
            }
        }
        self.focus_radius = outer_radius + 50.0;
    }
}

impl eframe::App for ViewerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Search bar
        egui::Panel::top("search_bar")
            .frame(
                egui::Frame::default().fill(egui::Color32::from_rgb(20,20,20))
                )
            // .stroke(egui::Stroke::new(1.0, egui::Color32::DARK_GRAY))
            .show(ui, |ui: &mut egui::Ui| {

            ui.horizontal(|ui: &mut egui::Ui| {
                ui.label("Search:");
                let search_response = ui.text_edit_singleline(&mut self.search);

                if !self.search.is_empty() {

                    let query = self.search.to_lowercase();

                    let matches: Vec<NodeIndex> = self.kept_nodes
                        .iter()
                        .copied()
                        .filter(|&idx| {
                            self.graph[idx]
                                .word
                                .to_lowercase()
                                .contains(&query)
                        })
                        .take(10)
                        .collect();

                    for idx in matches {
                        if ui.button(&self.graph[idx].word).clicked() {
                            self.select_node(idx, true);
                        }
                    }

                    if search_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if let Some(idx) = self.kept_nodes
                            .iter()
                            .copied()
                            .find(|&idx|
                                self.graph[idx]
                                    .word
                                    .eq_ignore_ascii_case(&self.search)
                            ) {
                            self.select_node(idx, true);
                        }
                    }
                }

            });

        });

        if self.show_edge_panel {
            egui::Panel::right("edge_panel")
                .default_size(250.0)
                .show(ui, |ui: &mut egui::Ui| {
                    ui.heading("Connections");
                    if let Some(node) = self.selected_node {
                        match self.diff {
                            false => ui.label(format!("{} ({})", self.graph[node].word, self.graph[node].count)),
                            true => ui.label(format!("{} ({:+.2e}%)", self.graph[node].word, self.graph[node].count as f32 / 10_000_000.0)),
                        };
                        ui.separator();
                        let mut connections: Vec<(String, i32)> =
                            self.graph
                                .edges(node)
                                .filter(|e|
                                    e.weight().abs() >= self.min_weight
                                )
                                .map(|edge| {
                                    let other =
                                        if edge.source() == node {
                                            edge.target()
                                        } else {
                                            edge.source()
                                        };
                                    (
                                        self.graph[other].word.clone(),
                                        *edge.weight()
                                    )
                                })
                                .collect();

                        // strongest connections first
                        connections.sort_by(|a, b|
                            b.1.cmp(&a.1)
                        );

                        egui::ScrollArea::vertical()
                            .show(ui, |ui| {
                                for (word, weight) in connections {
                                    // Find the actual node index from the word
                                    let target_idx = self.graph
                                        .node_indices()
                                        .find(|&idx| self.graph[idx].word == word);
                                    ui.horizontal(|ui| {
                                        if ui.button(&word).clicked() {
                                            if let Some(idx) = target_idx {
                                                self.select_node(idx, false);
                                            }
                                        }
                                        match self.diff {
                                            false => ui.label(format!("{}", weight)),
                                            true => ui.label(format!("{:+.2e}%", weight as f32 / 10_000_000.0)),
                                        }
                                    });
                                }
                            });
                    } else {
                        ui.label("Select a node to see connections");
                    }
                });
        }

        // Graph area
        egui::CentralPanel::default().show(ui, |ui: &mut egui::Ui| {

            let rect = ui.max_rect();
            self.viewport_size = rect.size();
            let center = rect.center().to_vec2();
            let painter = ui.painter();

            // Camera controls
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);

            if scroll != 0.0 {
                self.zoom = (self.zoom * (1.0 + scroll * 0.001)).clamp(0.05, 10.0);
            }

            let (secondary_down, pointer_delta) =
                ui.input(|i|
                    (
                        i.pointer.secondary_down(),
                        i.pointer.delta()
                    )
                );

            if secondary_down {
                self.pan += pointer_delta / self.zoom;
            }

            if ui.input(|i| i.key_pressed(egui::Key::N)) {
                self.hide_edges = !self.hide_edges;
            }

            if ui.input(|i| i.key_pressed(egui::Key::E)) {
                self.show_edge_panel = !self.show_edge_panel;
            }

            let response =
                ui.interact(rect, ui.id().with("background"), egui::Sense::click_and_drag());

            let pointer_world =
                ui.input(|i| i.pointer.hover_pos())
                .map(|p|
                    (p.to_vec2() - center)
                    / self.zoom
                    - self.pan
                );


            let hovered =
                pointer_world.and_then(|w| {
                    self.kept_nodes
                        .iter()
                        .copied()
                        .find(|&idx| {

                            let pos =
                                self.positions[&idx];

                            (pos - w).length()
                                <
                            node_radius(
                                self.graph[idx].count,
                                self.max_count,
                                self.size_exponent
                            )
                            + 3.0
                        })
                });

            // click selects, click empty clears
            if response.clicked() {
                self.selected_node = hovered;
                match hovered {
                    Some(idx) => self.select_node(idx, false),
                    None => {
                        self.selected_node = None;
                        self.neighbours.clear();
                    }
                }
            }

            let target = hovered.or(self.selected_node);

            // drag nodes
            if response.drag_started() {
                self.dragging_node = hovered;
            }

            if response.dragged() {
                if let Some(idx) = self.dragging_node {
                    if let Some(w) = pointer_world {
                        self.positions.insert(idx, w);
                    }
                }
            }

            if response.drag_stopped() {
                self.dragging_node = None;
            }

            // Calculate neighbourhood
            let node_set: HashSet<NodeIndex> =
                self.kept_nodes
                    .iter()
                    .copied()
                    .collect();

            // Draw edges
            if let Some(selected) = target {
                for edge in self.graph.edges(selected) {
                    if !node_set.contains(&edge.source()) || !node_set.contains(&edge.target()) {
                        continue;
                    }

                    if edge.weight().abs() < self.min_weight {
                        continue;
                    }

                    if let Some(selected) = target {
                        if edge.source() != selected && edge.target() != selected {
                            continue;
                        }
                    }

                    let colour = egui::Color32::from_gray(140);

                    let p1 = self.world_to_screen(self.positions[&edge.source()], center);
                    let p2 = self.world_to_screen(self.positions[&edge.target()], center);


                    painter.line_segment([p1, p2],
                        egui::Stroke::new(edge_stroke(edge.weight().abs(), self.max_weight), colour)
                    );
                }
            }

            // Draw nodes
            for &idx in &self.kept_nodes {
                let active = target == Some(idx) || self.neighbours.contains(&idx);
                let faded = self.dim_unrelated && target.is_some() && !active;
                let node = &self.graph[idx];

                let pos = self.world_to_screen(self.positions[&idx], center);

                let radius = 
                    node_radius(node.count.abs(), self.max_count, self.size_exponent) * self.zoom;

                let colour =
                    if target == Some(idx) {
                        egui::Color32::from_rgb(255, 180, 60)
                    } else if faded {
                        egui::Color32::from_rgba_premultiplied(35, 45, 60, 60)
                    } else if self.diff {
                        if node.count >= 0 {
                            egui::Color32::from_rgb(80, 200, 120) // green
                        } else {
                            egui::Color32::from_rgb(220, 80, 80) // red
                        }
                    } else {
                        egui::Color32::from_rgb(70, 130, 220)
                    };
                painter.circle_filled(pos, radius.max(1.5), colour);
            }

            let selected_screen_pos = target.map(|idx| {
                self.world_to_screen(self.positions[&idx], center)
            });

            // Draw labels
            for &idx in &self.kept_nodes {
                if target.is_some() && target != Some(idx) && !self.neighbours.contains(&idx) {
                    continue;
                }

                let node = &self.graph[idx];
                let pos = self.world_to_screen(self.positions[&idx], center);

                let offset = if let Some(selected_pos) = selected_screen_pos {
                    let delta = pos - selected_pos;

                    if delta.length_sq() > 0.0 {
                        delta.normalized() * 15.0
                    } else {
                        egui::vec2(15.0, 0.0)
                    }
                } else {
                    egui::vec2(12.0, 0.0)
                };
                
                match self.diff {
                    false => painter.text(
                        // pos + egui::vec2(12.0, 0.0),
                        pos + offset,
                        egui::Align2::LEFT_CENTER,
                        format!(
                            "{} ({})",
                            node.word,
                            node.count
                        ),
                        egui::FontId::proportional(14.0),
                        egui::Color32::WHITE
                        ),
                    true => painter.text(
                        // pos + egui::vec2(12.0, 0.0),
                        pos + offset,
                        egui::Align2::LEFT_CENTER,
                        format!(
                            "{} ({:+.2e}%)",
                            node.word,
                            node.count as f32 / 10_000_000.0,
                        ),
                        egui::FontId::proportional(14.0),
                        egui::Color32::WHITE
                        ),
                };
            }

            // HUD
            painter.text(
                rect.left_top()
                    + egui::vec2(8.0, 8.0),
                egui::Align2::LEFT_TOP,
                format!(
                    "{} nodes\n\
                     N: toggle edges ({})\n\
                     zoom: {:.2}x",
                    self.kept_nodes.len(),
                    if self.hide_edges {
                        "focused"
                    } else {
                        "all"
                    },
                    self.zoom
                ),
                egui::FontId::proportional(13.0),
                egui::Color32::LIGHT_GRAY,
            );

        });
    }
}

fn node_radius(count: i32, max_count: i32, exponant: f32) -> f32 {
    const MIN_R: f32 = 3.0;
    const MAX_R: f32 = 45.0;

    if max_count == 0 {
        return MIN_R;
    }

    let count = count.max(1) as f32;
    let max_count = max_count.max(1) as f32;

    // Compress huge ranges while preserving differences
    let t = (count / max_count)
        .powf(exponant);

    MIN_R + t * (MAX_R - MIN_R)
}

fn edge_stroke(weight: i32, max_weight: i32) -> f32 {
    const MIN_W: f32 = 0.5;
    const MAX_W: f32 = 6.0;
    if max_weight == 0 {
        return MIN_W;
    }
    let t = weight as f32 / max_weight as f32;
    MIN_W + t * (MAX_W - MIN_W)
}

pub fn plot_graph<'a>(path: &'a str, min_count: &'a i32, min_weight: &'a i32, spacing: &'a f32, size_exponant: &'a f32, diff: bool, filter: &'a str) -> Result<(), Box<dyn std::error::Error + 'a>> {
    let mut graph = node_graphing::load_graph(path)?;
    if !filter.is_empty() {
        let filter_set: HashSet<String> = load_word_set(filter);
        remove_words_from_graph(&mut graph, filter_set);
    }
    let min_count = *min_count;
    let min_weight = *min_weight;
    let spacing = *spacing;
    let size_exponant = *size_exponant;
    println!(
        "Loaded {} nodes, {} edges. Filtering with min_count={min_count}, min_weight={min_weight}",
        graph.node_count(),
        graph.edge_count()
    );

    // let native_options = eframe::NativeOptions::default();
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Word Co-occurrence Graph Viewer",
        native_options,
        Box::new(move |_cc| {
            Ok(Box::new(ViewerApp::new(graph, min_count, min_weight, spacing, size_exponant, diff)))
        }),
    )
    .map_err(move |e| format!("eframe error: {e}").into())
}
