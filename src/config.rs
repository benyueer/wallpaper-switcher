use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn load_dirs(config_path: &str) -> Vec<String> {
    let file = match File::open(config_path) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Failed to open config file: {}", config_path);
            return vec![];
        }
    };

    let reader = BufReader::new(file);

    reader
        .lines()
        .filter_map(|line| line.ok())
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}