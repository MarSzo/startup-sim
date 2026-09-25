//! Minimal `--key value` / `--flag` command-line parsing (no extra deps).

use std::collections::HashMap;
use std::str::FromStr;

pub struct Args {
    values: HashMap<String, String>,
}

impl Args {
    pub fn from_env() -> Args {
        Args::parse(std::env::args().skip(1))
    }

    pub fn parse(it: impl IntoIterator<Item = String>) -> Args {
        let mut values = HashMap::new();
        let mut it = it.into_iter().peekable();
        while let Some(a) = it.next() {
            if let Some(key) = a.strip_prefix("--") {
                if let Some((k, v)) = key.split_once('=') {
                    values.insert(k.to_string(), v.to_string());
                } else if it.peek().is_some_and(|n| !n.starts_with("--")) {
                    values.insert(key.to_string(), it.next().unwrap());
                } else {
                    values.insert(key.to_string(), String::new());
                }
            }
        }
        Args { values }
    }

    pub fn flag(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    pub fn get<T: FromStr>(&self, key: &str, default: T) -> T {
        match self.values.get(key) {
            Some(v) => v.parse().unwrap_or_else(|_| {
                eprintln!("invalid value for --{key}: {v}");
                std::process::exit(2);
            }),
            None => default,
        }
    }
}
