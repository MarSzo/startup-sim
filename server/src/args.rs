//! Minimal `--key value` / `--flag` command-line parsing (no extra deps).

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

pub struct Args {
    values: HashMap<String, String>,
}

/// A value that doesn't parse, e.g. `--max-players lots`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidArg {
    pub key: String,
    pub value: String,
}

impl fmt::Display for InvalidArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid value for --{}: {}", self.key, self.value)
    }
}

impl std::error::Error for InvalidArg {}

impl Args {
    pub fn from_env() -> Args {
        Args::parse(std::env::args().skip(1))
    }

    pub fn parse(it: impl IntoIterator<Item = String>) -> Args {
        let mut values = HashMap::new();
        let mut it = it.into_iter().peekable();
        while let Some(a) = it.next() {
            let Some(key) = a.strip_prefix("--") else { continue };
            let (k, v) = match key.split_once('=') {
                Some((k, v)) => (k.to_string(), v.to_string()),
                None => {
                    let v = it.next_if(|n| !n.starts_with("--")).unwrap_or_default();
                    (key.to_string(), v)
                }
            };
            values.insert(k, v);
        }
        Args { values }
    }

    pub fn flag(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// The parsed value of `--key`, or `default` when it's not given.
    ///
    /// # Errors
    /// When the value doesn't parse as `T`.
    pub fn try_get<T: FromStr>(&self, key: &str, default: T) -> Result<T, InvalidArg> {
        match self.values.get(key) {
            Some(v) => v.parse().map_err(|_| InvalidArg { key: key.into(), value: v.clone() }),
            None => Ok(default),
        }
    }

    /// Like [`Args::try_get`], but exits with status 2 on a bad value (for
    /// small tools).
    pub fn get<T: FromStr>(&self, key: &str, default: T) -> T {
        self.try_get(key, default).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Args {
        Args::parse(s.split_whitespace().map(String::from))
    }

    #[test]
    fn values_flags_and_equals_forms() {
        let a = args("--bind 127.0.0.1:1 --treats --lag-ms=40 stray");
        assert_eq!(a.str("bind"), Some("127.0.0.1:1"));
        assert!(a.flag("treats"));
        assert_eq!(a.try_get("lag-ms", 0u64), Ok(40));
        assert_eq!(a.try_get("missing", 7u32), Ok(7));
    }

    #[test]
    fn a_flag_before_another_option_has_no_value() {
        let a = args("--treats --weather rain");
        assert_eq!(a.str("treats"), Some(""));
        assert_eq!(a.str("weather"), Some("rain"));
    }

    #[test]
    fn bad_values_are_errors() {
        let e = args("--max-players lots").try_get("max-players", 0usize).unwrap_err();
        assert_eq!(e.to_string(), "invalid value for --max-players: lots");
    }
}
