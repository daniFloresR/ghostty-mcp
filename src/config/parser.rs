use std::collections::BTreeMap;

/// A single line from the config file, preserving format.
#[derive(Debug, Clone)]
pub enum ConfigLine {
    /// A comment line (starts with #)
    Comment(String),
    /// An empty line
    Empty,
    /// A key-value pair: key = value (with original formatting)
    KeyValue { key: String, value: String },
}

/// Parsed config file preserving structure for round-trip editing.
#[derive(Debug, Clone)]
pub struct ConfigFile {
    pub lines: Vec<ConfigLine>,
    pub path: String,
}

impl ConfigFile {
    /// Parse a config file from its contents.
    pub fn parse(contents: &str, path: &str) -> Self {
        let lines = contents
            .lines()
            .map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    ConfigLine::Empty
                } else if trimmed.starts_with('#') {
                    ConfigLine::Comment(line.to_string())
                } else if let Some(eq_pos) = line.find('=') {
                    let key = line[..eq_pos].trim().to_string();
                    let value = line[eq_pos + 1..].trim().to_string();
                    ConfigLine::KeyValue { key, value }
                } else {
                    // Treat malformed lines as comments
                    ConfigLine::Comment(line.to_string())
                }
            })
            .collect();

        ConfigFile {
            lines,
            path: path.to_string(),
        }
    }

    /// Get all key-value pairs as a map. For repeated keys, values are collected into a Vec.
    pub fn to_map(&self) -> BTreeMap<String, Vec<String>> {
        let mut map = BTreeMap::new();
        for line in &self.lines {
            if let ConfigLine::KeyValue { key, value } = line {
                map.entry(key.clone())
                    .or_insert_with(Vec::new)
                    .push(value.clone());
            }
        }
        map
    }

    /// Get the value of a specific option. Returns the last set value.
    pub fn get(&self, option: &str) -> Option<String> {
        self.lines
            .iter()
            .rev()
            .find_map(|line| match line {
                ConfigLine::KeyValue { key, value } if key == option => Some(value.clone()),
                _ => None,
            })
    }

}

impl std::fmt::Display for ConfigFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            match line {
                ConfigLine::Comment(s) => write!(f, "{}", s)?,
                ConfigLine::Empty => {}
                ConfigLine::KeyValue { key, value } => {
                    if value.is_empty() {
                        write!(f, "{key} =")?;
                    } else {
                        write!(f, "{key} = {value}")?;
                    }
                }
            }
        }
        writeln!(f)?;
        Ok(())
    }
}

/// Read and parse the config file at the given path.
pub fn read_config(path: &str) -> anyhow::Result<ConfigFile> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Empty config file — Ghostty uses defaults
            String::new()
        }
        Err(e) => return Err(e.into()),
    };
    Ok(ConfigFile::parse(&contents, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_config() {
        let config = ConfigFile::parse("", "/test");
        assert!(config.lines.is_empty());
        assert!(config.to_map().is_empty());
    }

    #[test]
    fn parse_key_value_pairs() {
        let config = ConfigFile::parse("font-size = 14\ntheme = dark", "/test");
        assert_eq!(config.lines.len(), 2);
        assert_eq!(config.get("font-size"), Some("14".to_string()));
        assert_eq!(config.get("theme"), Some("dark".to_string()));
    }

    #[test]
    fn parse_comments_and_empty_lines() {
        let config = ConfigFile::parse("# This is a comment\n\nfont-size = 14", "/test");
        assert_eq!(config.lines.len(), 3);
        assert!(matches!(&config.lines[0], ConfigLine::Comment(s) if s.contains("comment")));
        assert!(matches!(&config.lines[1], ConfigLine::Empty));
        assert!(matches!(&config.lines[2], ConfigLine::KeyValue { key, .. } if key == "font-size"));
    }

    #[test]
    fn parse_key_with_empty_value() {
        let config = ConfigFile::parse("font-family =", "/test");
        assert_eq!(config.get("font-family"), Some("".to_string()));
    }

    #[test]
    fn parse_value_with_equals_sign() {
        // Ghostty keybinds use = in value: keybind = ctrl+a=new_tab
        let config = ConfigFile::parse("keybind = ctrl+a=new_tab", "/test");
        assert_eq!(config.get("keybind"), Some("ctrl+a=new_tab".to_string()));
    }

    #[test]
    fn parse_repeated_keys() {
        let config = ConfigFile::parse("font-family = Fira Code\nfont-family = JetBrains Mono", "/test");
        let map = config.to_map();
        assert_eq!(map["font-family"].len(), 2);
        // get() returns the last value
        assert_eq!(config.get("font-family"), Some("JetBrains Mono".to_string()));
    }

    #[test]
    fn parse_malformed_line_treated_as_comment() {
        let config = ConfigFile::parse("this has no equals sign", "/test");
        assert_eq!(config.lines.len(), 1);
        assert!(matches!(&config.lines[0], ConfigLine::Comment(_)));
        assert!(config.to_map().is_empty());
    }

    #[test]
    fn to_string_round_trip() {
        let input = "# Comment\n\nfont-size = 14\ntheme = dark\n";
        let config = ConfigFile::parse(input, "/test");
        assert_eq!(config.to_string(), input);
    }

    #[test]
    fn read_config_nonexistent_file() {
        let config = read_config("/nonexistent/path/to/config").unwrap();
        assert!(config.lines.is_empty());
    }

    #[test]
    fn read_config_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config");
        std::fs::write(&path, "font-size = 16\n").unwrap();
        let config = read_config(path.to_str().unwrap()).unwrap();
        assert_eq!(config.get("font-size"), Some("16".to_string()));
    }
}
