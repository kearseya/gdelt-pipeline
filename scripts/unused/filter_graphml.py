#!/usr/bin/env python3
"""
Filter a GraphML graph (as exported by export_graph_graphml) by a minimum
node "count" attribute, without needing to rebuild it from Rust.

Usage:
    python filter_graphml.py input.graphml output.graphml 5

Keeps only nodes whose "count" attribute is greater than the given
min_count (matching the same `count > min_count` semantics used in the
Rust exporter), and drops any edge that touches a removed node.
"""

import sys
import networkx as nx


def filter_graphml(input_path: str, output_path: str, min_count: int) -> None:
    graph = nx.read_graphml(input_path)

    print(f"Loaded: {graph.number_of_nodes()} nodes, {graph.number_of_edges()} edges")

    nodes_to_drop = [
        node_id
        for node_id, attrs in graph.nodes(data=True)
        if attrs.get("count", 0) <= min_count
    ]
    graph.remove_nodes_from(nodes_to_drop)

    print(
        f"After min_count={min_count}: "
        f"{graph.number_of_nodes()} nodes, {graph.number_of_edges()} edges "
        f"({len(nodes_to_drop)} nodes dropped)"
    )

    nx.write_graphml(graph, output_path)
    print(f"Wrote {output_path}")


if __name__ == "__main__":
    if len(sys.argv) != 4:
        print(f"Usage: {sys.argv[0]} <input.graphml> <output.graphml> <min_count>")
        sys.exit(1)

    input_path, output_path, min_count_arg = sys.argv[1], sys.argv[2], sys.argv[3]
    filter_graphml(input_path, output_path, int(min_count_arg))
