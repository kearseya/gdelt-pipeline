use std::{
    fs,
    fs::File,
    path::Path,
    collections::{HashSet, HashMap},
    io::{BufRead, BufReader, Read},
    sync::atomic::{AtomicUsize, Ordering},
    error::Error,
};
use csv::ReaderBuilder;
use regex::Regex;
use rayon::prelude::*;
use zip::ZipArchive;

use crate::node_graphing;

pub fn load_word_set(path: &str) -> HashSet<String> {
    let file = File::open(path).expect("Could not open word file");
    let reader = BufReader::new(file);

    reader
        .lines()
        .filter_map(|line| line.ok())
        .map(|word| word.trim().to_lowercase())
        .filter(|word| !word.is_empty())
        .collect()
}

fn extract_domain_name(host: &str) -> Option<String> {
    let domain = psl::domain(host.as_bytes())?;
    let domain_str = std::str::from_utf8(domain.as_bytes()).ok()?;
    let suffix_str = std::str::from_utf8(domain.suffix().as_bytes()).ok()?;

    domain_str
        .strip_suffix(suffix_str)
        .and_then(|s| s.strip_suffix('.'))
        .map(|s| s.to_string())
}

fn list_csv_files(directory: &str) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(fs::read_dir(directory)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("csv") || ext.eq_ignore_ascii_case("zip"))
                .unwrap_or(false)
        })
        .filter_map(|path| path.to_str().map(String::from))
        .collect())
}

fn is_valid_word(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }

    if !word.chars().all(|c| c.is_alphanumeric()) {
        return false;
    }

    // Large numeric IDs
    if word.chars().all(|c| c.is_ascii_digit()) && word.len() >= 6 {
        return false;
    }

    // Hex-like identifiers
    if word.len() >= 4 {
        let is_hex = word.chars().all(|c| c.is_ascii_hexdigit());
        let has_digit = word.chars().any(|c| c.is_ascii_digit());
        let has_letter = word.chars().any(|c| c.is_ascii_alphabetic());

        if is_hex && has_digit && has_letter {
            return false;
        }
    }

    // Long numeric suffixes
    if word
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .count() >= 6
    {
        return false;
    }

    true
}

#[allow(dead_code)]
fn is_valid_url_segment(segment: &str) -> bool {
    if segment.is_empty() {
        return false;
    }

    let digits: String = segment
        .chars()
        .rev()
        .take_while(|c| c.is_numeric())
        .collect();

    let digit_suffix_len = digits.len();

    let digit_count = segment.chars().filter(|c| c.is_numeric()).count();
    let letter_count = segment.chars().filter(|c| c.is_alphabetic()).count();

    // Reject pure numbers
    if digit_count == segment.len() {
        return false;
    }

    // Reject words ending in large numeric IDs:
    if digit_suffix_len >= 5 {
        return false;
    }

    // Reject mostly numeric long IDs
    if segment.len() > 20 && digit_count > letter_count {
        return false;
    }

    // Reject short hex-like hashes
    if segment.len() <= 12 && segment.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }

    true
}

pub struct UrlAnalyser {
    domain_set: HashSet<String>,
    include_set: HashSet<String>,
    exclude_set: HashSet<String>,
    include_domains: bool,
    filter_column: usize,
    filter_value: String,
    dedup: bool,
    total_lines_parsed: AtomicUsize,
    used_lines_parsed: AtomicUsize,
}

impl UrlAnalyser {
    pub fn new(
        domain_set: HashSet<String>,
        include_set: HashSet<String>,
        exclude_set: HashSet<String>,
        include_domains: bool,
        filter_column: usize,
        filter_value: String,
        dedup: bool,
    ) -> Self {
        UrlAnalyser {
            domain_set,
            include_set,
            exclude_set,
            include_domains,
            filter_column,
            filter_value,
            dedup,
            total_lines_parsed: AtomicUsize::new(0),
            used_lines_parsed: AtomicUsize::new(0),
        }
    }

    // For rank command
    pub fn count_from_file(&self, path: &str, counts: &mut HashMap<String, usize>, verbose: bool) {
        let urls = self.read_from_file(path);
        let urls = self.include_words(&urls);
        let urls = self.exclude_words(&urls);
        let urls = self.dedup_words(urls);
        self.used_lines_parsed.fetch_add(urls.len(), Ordering::Relaxed);

        for url in &urls {
            for word in url {
                *counts.entry(word.clone()).or_insert(0) += 1;
            }
            if verbose {
                println!("{}", url.join(","));
            }
        }
    }

    pub fn count_from_directory(&self, directory: &str, counts: &mut HashMap<String, usize>, verbose: bool) {
        println!("Reading directory: {}", directory);

        let csv_paths: Vec<String> = list_csv_files(directory).unwrap();

        let merged = csv_paths
            .par_iter()
            .map(|path| {
                let mut local = HashMap::<String, usize>::new();
                self.count_from_file(path, &mut local, verbose);
                local
            })
            .reduce(HashMap::new, |mut a, b| {
                for (k, v) in b {
                    *a.entry(k).or_insert(0) += v;
                }
                a
            });
        *counts = merged;
    }

    // Reading for node command
    pub fn read_from_file(&self, path: &str) -> Vec<Vec<String>> {
        if path.to_lowercase().ends_with(".zip") {
            self.read_from_zip(path)
        } else {
            self.read_from_plain_file(path)
        }
    }

    fn read_from_plain_file(&self, path: &str) -> Vec<Vec<String>> {
        println!("Processing data path: {}", path);

        let mut reader = ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_path(path)
            .unwrap();

        self.consume_csv_records(&mut reader)
    }

    fn read_from_zip(&self, path: &str) -> Vec<Vec<String>> {
        println!("Processing zip archive: {}", path);

        let file = File::open(path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        let mut urls = Vec::new();

        for i in 0..archive.len() {
            let zip_entry = archive.by_index(i).unwrap();
            println!("  reading entry: {}", zip_entry.name());

            let mut reader = ReaderBuilder::new()
                .delimiter(b'\t')
                .has_headers(false)
                .from_reader(zip_entry);

            urls.extend(self.consume_csv_records(&mut reader));
        }

        urls
    }

    // Shared for zip and csv
    fn consume_csv_records<R: Read>(&self, reader: &mut csv::Reader<R>) -> Vec<Vec<String>> {
        let url_column = 57;
        let mut urls = Vec::new();

        for result in reader.records() {
            self.total_lines_parsed.fetch_add(1, Ordering::Relaxed);
            let record = result.unwrap();

            let passes_filter = record.len() > self.filter_column
                && record.get(self.filter_column) == Some(self.filter_value.as_str());

            if passes_filter && record.len() > url_column {
                let url: String = record[url_column].to_string();
                let url_word_list: Vec<String> = self.url_to_words(&url);
                if url_word_list.len() > 3 {
                    urls.push(url_word_list);
                }
            }
        }

        urls
    }

    pub fn read_from_directory(&self, directory: &str) -> Vec<Vec<String>> {
        println!("Reading directory: {}", directory);

        let csv_paths: Vec<String> = list_csv_files(directory).unwrap();

        csv_paths
            .par_iter()
            .flat_map(|path| self.read_from_file(path))
            .collect()
    }

    // From file
    pub fn process_file_to_graph(&self, path: &str, output_path: &std::path::Path) -> Result<(), Box<dyn Error>> {
        let rows = self.read_from_file(path); // now zip-aware too

        let rows: Vec<Vec<String>> = self.include_words(&rows);
        let rows: Vec<Vec<String>> = self.exclude_words(&rows);
        let rows: Vec<Vec<String>> = self.dedup_words(rows);
        self.used_lines_parsed.fetch_add(rows.len(), Ordering::Relaxed);

        let graph = node_graphing::build_word_graph(rows)?;

        let _ = node_graphing::save_graph(&graph, output_path.to_str().unwrap())?;
        Ok(())
    }

    pub fn process_directory_to_graphs(&self, directory: &str, output_dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>, Box<dyn Error>> {
        std::fs::create_dir_all(output_dir)?;

        let csv_paths: Vec<String> = list_csv_files(directory).unwrap();

        let output_paths: Vec<std::path::PathBuf> = csv_paths
            .par_iter()
            .enumerate()
            .map(|(i, path)| {
                let output_path = output_dir.join(format!("graph_{i}.bin"));
                self.process_file_to_graph(path, &output_path).unwrap();
                output_path
            })
            .collect();

        Ok(output_paths)
    }

    // Check input to read
    pub fn check_path(&self, input: &str) -> Vec<Vec<String>> {
        let path = Path::new(input);

        if path.is_file() {
            self.read_from_file(input)
        } else if path.is_dir() {
            self.read_from_directory(input)
        } else {
            println!("Not a valid path");
            Vec::new()
        }
    }

    fn url_to_words(&self, url: &str) -> Vec<String> {
        if !self.domain_set.is_empty() {
            let matches = self.domain_set.iter().any(|d| url.contains(d.as_str()));
            if !matches {
                return Vec::new();
            }
        }

        let url_regex = Regex::new(r"://([^/]+)(?:/(.*))?").unwrap();
        let caps = match url_regex.captures(url) {
            Some(c) => c,
            None => return Vec::new(),
        };
        let host = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let path = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let domain = extract_domain_name(host);

        let mut words = Vec::new();
        if self.include_domains {
            if let Some(d) = &domain {
                words.push(d.clone());
            }
        }
        for part in path.split('/') {
            for word in part.split(|c: char| c == '-' || c == '_' || c == '.') {
                if is_valid_word(word) {
                    words.push(word.to_string().to_lowercase());
                }
            }
        }
        words
    }

    // Filtering

    pub fn include_words(&self, data: &[Vec<String>]) -> Vec<Vec<String>> {
        if self.include_set.is_empty() {
            return data.to_vec();
        }
        data.iter()
            .map(|row| {
                row.iter()
                    .filter(|word| self.include_set.contains(*word))
                    .cloned()
                    .collect()
            })
            .collect()
    }

    pub fn exclude_words(&self, data: &[Vec<String>]) -> Vec<Vec<String>> {
        if self.exclude_set.is_empty() {
            return data.to_vec();
        }
        data.iter()
            .map(|row| {
                row.iter()
                    .filter(|word| !self.exclude_set.contains(*word))
                    .cloned()
                    .collect()
            })
            .collect()
    }

    pub fn dedup_words(&self, data: Vec<Vec<String>>) -> Vec<Vec<String>> {
        if !self.dedup {
            return data;
        }
        let mut seen: HashSet<Vec<String>> = HashSet::new();
        data.into_iter().filter(|row| seen.insert(row.clone())).collect()
    }

    // Running //
    // rank
    pub fn count_words(&self, input: &str, verbose: bool) -> HashMap<String, usize> {
        let path = Path::new(input);
        let mut counts = HashMap::<String, usize>::new();

        if path.is_file() {
            self.count_from_file(input, &mut counts, verbose)
        } else if path.is_dir() {
            self.count_from_directory(input, &mut counts, verbose)
        } else {
            println!("Not a valid path");
        }
        counts
    }

    // node
    pub fn analyse(&self, input: &str) -> Vec<Vec<String>> {
        let url_words = self.check_path(input);
        let url_words = self.include_words(&url_words);
        let url_words = self.exclude_words(&url_words);
        let url_words = self.dedup_words(url_words);
        url_words
    }

    // "Get" functions

    pub fn lines_parsed(&self) -> usize {
        self.total_lines_parsed.load(Ordering::Relaxed)
    }

    pub fn lines_used(&self) -> usize {
        self.used_lines_parsed.load(Ordering::Relaxed)
    }

}
