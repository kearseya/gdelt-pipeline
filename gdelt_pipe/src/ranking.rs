use std::{
    collections::{HashMap, HashSet},
};
use crate::csv_parsing;


pub fn display_top_words(counts: &HashMap<String, usize>, x: usize) {
    let mut sorted_words: Vec<(&String, &usize)> = counts.iter().collect();

    sorted_words.sort_by(|a, b| b.1.cmp(a.1));

    println!("Top {} words:", x);

    for (word, count) in sorted_words.iter().take(x) {
        println!("{}: {}", word, count);
    }
}

pub fn show_top_words(
    input: &str,
    include: &str,
    exclude: &str,
    n: &i32,
    domains: &str,
    include_domains: bool,
    filter_column: usize,
    filter_value: &str,
    dedup: bool,
    verbose: bool,
) {
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
    let word_counts: HashMap<String, usize> = analyser.count_words(input, verbose);

    println!("Total lines parsed: {}", analyser.lines_parsed());
    println!("Lines used: {}", analyser.lines_used());
    println!("Number of words counted: {}", word_counts.len());

    display_top_words(&word_counts, *n as usize);
}
