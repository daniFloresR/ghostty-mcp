use std::collections::{BTreeMap, HashSet};
use std::path::Path;

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

    /// Get all values for a specific option. Useful for repeatable options like keybind.
    pub fn get_all(&self, option: &str) -> Vec<String> {
        self.lines
            .iter()
            .filter_map(|line| match line {
                ConfigLine::KeyValue { key, value } if key == option => Some(value.clone()),
                _ => None,
            })
            .collect()
    }

    /// Get the value of a specific option. Returns the last set value.
    pub fn get(&self, option: &str) -> Option<String> {
        self.lines.iter().rev().find_map(|line| match line {
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

/// A config value tagged with its origin file.
#[derive(Debug, Clone)]
pub struct AnnotatedValue {
    pub value: String,
    pub source_file: String,
}

/// A resolved include directive from a config file.
#[derive(Debug, Clone)]
pub struct IncludedFile {
    /// The raw directive value (e.g. "?fonts" or "keybinds")
    pub directive: String,
    /// The resolved absolute path
    pub resolved_path: String,
    /// Whether the include was optional (prefixed with `?`)
    pub optional: bool,
    /// The parsed config, None if optional and missing
    pub config: Option<ConfigFile>,
    /// Which file included this one
    pub included_from: String,
}

/// The full config hierarchy: primary file plus all resolved includes.
#[derive(Debug, Clone)]
pub struct ConfigTree {
    pub primary: ConfigFile,
    pub includes: Vec<IncludedFile>,
}

impl ConfigTree {
    /// Get the effective value for a key (last wins across load order).
    /// Load order: primary first, then includes in declaration order (depth-first).
    /// Later files override earlier ones.
    pub fn get(&self, key: &str) -> Option<AnnotatedValue> {
        // Check includes in reverse order (last wins)
        for inc in self.includes.iter().rev() {
            if let Some(ref config) = inc.config {
                if let Some(value) = config.get(key) {
                    return Some(AnnotatedValue {
                        value,
                        source_file: config.path.clone(),
                    });
                }
            }
        }
        // Fall back to primary
        self.primary.get(key).map(|value| AnnotatedValue {
            value,
            source_file: self.primary.path.clone(),
        })
    }

    /// Get all values for a key across all files, with source annotations.
    pub fn get_all(&self, key: &str) -> Vec<AnnotatedValue> {
        let mut results = Vec::new();
        // Primary first
        for value in self.primary.get_all(key) {
            results.push(AnnotatedValue {
                value,
                source_file: self.primary.path.clone(),
            });
        }
        // Then includes in order
        for inc in &self.includes {
            if let Some(ref config) = inc.config {
                for value in config.get_all(key) {
                    results.push(AnnotatedValue {
                        value,
                        source_file: config.path.clone(),
                    });
                }
            }
        }
        results
    }

    /// Get all options as an annotated map grouped by key.
    pub fn to_annotated_map(&self) -> BTreeMap<String, Vec<AnnotatedValue>> {
        let mut map: BTreeMap<String, Vec<AnnotatedValue>> = BTreeMap::new();
        // Primary first
        for line in &self.primary.lines {
            if let ConfigLine::KeyValue { key, value } = line {
                map.entry(key.clone()).or_default().push(AnnotatedValue {
                    value: value.clone(),
                    source_file: self.primary.path.clone(),
                });
            }
        }
        // Then includes in order
        for inc in &self.includes {
            if let Some(ref config) = inc.config {
                for line in &config.lines {
                    if let ConfigLine::KeyValue { key, value } = line {
                        map.entry(key.clone()).or_default().push(AnnotatedValue {
                            value: value.clone(),
                            source_file: config.path.clone(),
                        });
                    }
                }
            }
        }
        map
    }

    /// Check if any file in the tree sets a `theme` to a non-empty value.
    pub fn has_theme_set(&self) -> bool {
        if let Some(v) = self.primary.get("theme") {
            if !v.is_empty() {
                return true;
            }
        }
        for inc in &self.includes {
            if let Some(ref config) = inc.config {
                if let Some(v) = config.get("theme") {
                    if !v.is_empty() {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Read and parse a config file and all its includes recursively.
pub fn read_config_tree(path: &str) -> anyhow::Result<ConfigTree> {
    let primary = read_config(path)?;
    let mut includes = Vec::new();
    let mut visited = HashSet::new();

    // Canonicalize the primary path for cycle detection
    let canonical_primary =
        std::fs::canonicalize(path).unwrap_or_else(|_| std::path::PathBuf::from(path));
    visited.insert(canonical_primary.to_string_lossy().to_string());

    let primary_dir = Path::new(path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    // Ghostty processes config-file directives as a queue: a file's includes
    // load after the entire file containing them, in declaration order, and
    // includes discovered while loading append to the end. The resulting
    // flat order is the load order, so later entries override earlier ones.
    let mut queue: std::collections::VecDeque<(String, std::path::PathBuf, String)> = primary
        .get_all("config-file")
        .into_iter()
        .map(|d| (d, primary_dir.clone(), path.to_string()))
        .collect();

    while let Some((directive, base_dir, included_from)) = queue.pop_front() {
        let (optional, raw_path) = parse_include_directive(&directive);
        if raw_path.is_empty() {
            continue;
        }

        // Resolve path relative to the declaring file's directory
        let resolved = if Path::new(&raw_path).is_absolute() {
            std::path::PathBuf::from(&raw_path)
        } else {
            base_dir.join(&raw_path)
        };
        let resolved_str = resolved.to_string_lossy().to_string();

        // Cycle detection using canonical paths
        let canonical = std::fs::canonicalize(&resolved).unwrap_or_else(|_| resolved.clone());
        let canonical_str = canonical.to_string_lossy().to_string();

        if visited.contains(&canonical_str) {
            tracing::warn!(
                "Skipping circular include: {} -> {}",
                included_from,
                resolved_str
            );
            continue;
        }
        visited.insert(canonical_str);

        // Try to read the included file
        match std::fs::read_to_string(&resolved) {
            Ok(contents) => {
                let inc_config = ConfigFile::parse(&contents, &resolved_str);
                let inc_dir = resolved.parent().unwrap_or(Path::new(".")).to_path_buf();

                for nested in inc_config.get_all("config-file") {
                    queue.push_back((nested, inc_dir.clone(), resolved_str.clone()));
                }

                includes.push(IncludedFile {
                    directive: directive.clone(),
                    resolved_path: resolved_str,
                    optional,
                    config: Some(inc_config),
                    included_from,
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && optional => {
                includes.push(IncludedFile {
                    directive: directive.clone(),
                    resolved_path: resolved_str,
                    optional,
                    config: None,
                    included_from,
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(anyhow::anyhow!(
                    "Included config file not found: {} (included from {})",
                    resolved_str,
                    included_from
                ));
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(ConfigTree { primary, includes })
}

/// Parse a config-file directive value: a leading '?' marks the include
/// optional, and the remaining path may be wrapped in double quotes (the
/// quoting Ghostty requires for paths starting with a literal '?').
fn parse_include_directive(directive: &str) -> (bool, String) {
    let (optional, rest) = match directive.strip_prefix('?') {
        Some(stripped) => (true, stripped),
        None => (false, directive),
    };
    let rest = rest.trim();
    let unquoted = rest
        .strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .unwrap_or(rest);
    (optional, unquoted.to_string())
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
        let config = ConfigFile::parse(
            "font-family = Fira Code\nfont-family = JetBrains Mono",
            "/test",
        );
        let map = config.to_map();
        assert_eq!(map["font-family"].len(), 2);
        // get() returns the last value
        assert_eq!(
            config.get("font-family"),
            Some("JetBrains Mono".to_string())
        );
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
    fn get_all_returns_all_values() {
        let config = ConfigFile::parse(
            "keybind = ctrl+a=new_tab\nkeybind = ctrl+b=new_window\n",
            "/test",
        );
        let all = config.get_all("keybind");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0], "ctrl+a=new_tab");
        assert_eq!(all[1], "ctrl+b=new_window");
    }

    #[test]
    fn get_all_empty_for_missing_key() {
        let config = ConfigFile::parse("font-size = 14\n", "/test");
        let all = config.get_all("keybind");
        assert!(all.is_empty());
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

    // --- ConfigTree tests ---

    #[test]
    fn tree_single_file_no_includes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config");
        std::fs::write(&path, "font-size = 14\ntheme = dark\n").unwrap();

        let tree = read_config_tree(path.to_str().unwrap()).unwrap();
        assert!(tree.includes.is_empty());
        assert_eq!(tree.get("font-size").unwrap().value, "14");
        assert_eq!(tree.get("theme").unwrap().value, "dark");
        assert!(tree.get("nonexistent").is_none());
    }

    #[test]
    fn tree_follows_include_last_wins() {
        let dir = tempfile::tempdir().unwrap();
        let fonts_path = dir.path().join("fonts");
        std::fs::write(&fonts_path, "font-size = 16\n").unwrap();

        let primary_path = dir.path().join("config");
        std::fs::write(&primary_path, "font-size = 14\nconfig-file = fonts\n").unwrap();

        let tree = read_config_tree(primary_path.to_str().unwrap()).unwrap();
        assert_eq!(tree.includes.len(), 1);
        // Include overrides primary (loaded after)
        let effective = tree.get("font-size").unwrap();
        assert_eq!(effective.value, "16");
        assert!(effective.source_file.contains("fonts"));
    }

    #[test]
    fn tree_optional_missing_file_no_error() {
        let dir = tempfile::tempdir().unwrap();
        let primary_path = dir.path().join("config");
        std::fs::write(&primary_path, "font-size = 14\nconfig-file = ?missing\n").unwrap();

        let tree = read_config_tree(primary_path.to_str().unwrap()).unwrap();
        assert_eq!(tree.includes.len(), 1);
        assert!(tree.includes[0].optional);
        assert!(tree.includes[0].config.is_none());
        // Primary value still works
        assert_eq!(tree.get("font-size").unwrap().value, "14");
    }

    #[test]
    fn tree_required_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let primary_path = dir.path().join("config");
        std::fs::write(&primary_path, "config-file = nonexistent\n").unwrap();

        let result = read_config_tree(primary_path.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn tree_cycle_detection() {
        let dir = tempfile::tempdir().unwrap();
        let a_path = dir.path().join("a");
        let b_path = dir.path().join("b");

        // A includes B, B includes A
        std::fs::write(&a_path, "config-file = b\nfont-size = 14\n").unwrap();
        std::fs::write(&b_path, "config-file = a\ntheme = dark\n").unwrap();

        let tree = read_config_tree(a_path.to_str().unwrap()).unwrap();
        // B is included, but B's include of A is skipped (cycle)
        assert_eq!(tree.includes.len(), 1);
        assert_eq!(tree.get("font-size").unwrap().value, "14");
        assert_eq!(tree.get("theme").unwrap().value, "dark");
    }

    #[test]
    fn tree_nested_includes_precedence() {
        let dir = tempfile::tempdir().unwrap();
        let c_path = dir.path().join("c");
        std::fs::write(&c_path, "font-size = 18\n").unwrap();

        let b_path = dir.path().join("b");
        std::fs::write(&b_path, "font-size = 16\nconfig-file = c\n").unwrap();

        let a_path = dir.path().join("a");
        std::fs::write(&a_path, "font-size = 14\nconfig-file = b\n").unwrap();

        let tree = read_config_tree(a_path.to_str().unwrap()).unwrap();
        // Ghostty load order: a, then a's includes (b), then b's includes
        // (c). Files loaded later override earlier ones, so c wins.
        let effective = tree.get("font-size").unwrap();
        assert_eq!(effective.value, "18");
        assert!(effective.source_file.contains('c'));
    }

    #[test]
    fn tree_multiple_includes_order() {
        let dir = tempfile::tempdir().unwrap();
        let fonts_path = dir.path().join("fonts");
        std::fs::write(&fonts_path, "font-size = 16\n").unwrap();

        let keys_path = dir.path().join("keybinds");
        std::fs::write(&keys_path, "font-size = 20\n").unwrap();

        let primary_path = dir.path().join("config");
        std::fs::write(
            &primary_path,
            "font-size = 14\nconfig-file = fonts\nconfig-file = keybinds\n",
        )
        .unwrap();

        let tree = read_config_tree(primary_path.to_str().unwrap()).unwrap();
        assert_eq!(tree.includes.len(), 2);
        // keybinds is last, so its value wins
        let effective = tree.get("font-size").unwrap();
        assert_eq!(effective.value, "20");
    }

    #[test]
    fn tree_relative_path_resolution() {
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("sub");
        std::fs::create_dir(&subdir).unwrap();

        let nested_path = subdir.join("nested");
        std::fs::write(&nested_path, "theme = dark\n").unwrap();

        // Include from subdir references a file relative to subdir
        let mid_path = subdir.join("mid");
        std::fs::write(&mid_path, "config-file = nested\n").unwrap();

        let primary_path = dir.path().join("config");
        std::fs::write(&primary_path, "config-file = sub/mid\n").unwrap();

        let tree = read_config_tree(primary_path.to_str().unwrap()).unwrap();
        assert_eq!(tree.get("theme").unwrap().value, "dark");
    }

    #[test]
    fn tree_get_all_annotated() {
        let dir = tempfile::tempdir().unwrap();
        let keys_path = dir.path().join("keybinds");
        std::fs::write(&keys_path, "keybind = ctrl+b=new_window\n").unwrap();

        let primary_path = dir.path().join("config");
        std::fs::write(
            &primary_path,
            "keybind = ctrl+a=new_tab\nconfig-file = keybinds\n",
        )
        .unwrap();

        let tree = read_config_tree(primary_path.to_str().unwrap()).unwrap();
        let all = tree.get_all("keybind");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].value, "ctrl+a=new_tab");
        assert!(all[0].source_file.contains("config"));
        assert_eq!(all[1].value, "ctrl+b=new_window");
        assert!(all[1].source_file.contains("keybinds"));
    }

    #[test]
    fn tree_has_theme_set() {
        let dir = tempfile::tempdir().unwrap();
        let theme_path = dir.path().join("theme");
        std::fs::write(&theme_path, "theme = catppuccin\n").unwrap();

        let primary_path = dir.path().join("config");
        std::fs::write(&primary_path, "config-file = theme\n").unwrap();

        let tree = read_config_tree(primary_path.to_str().unwrap()).unwrap();
        assert!(tree.has_theme_set());
    }

    #[test]
    fn tree_has_theme_not_set() {
        let dir = tempfile::tempdir().unwrap();
        let primary_path = dir.path().join("config");
        std::fs::write(&primary_path, "font-size = 14\n").unwrap();

        let tree = read_config_tree(primary_path.to_str().unwrap()).unwrap();
        assert!(!tree.has_theme_set());
    }

    #[test]
    fn tree_to_annotated_map() {
        let dir = tempfile::tempdir().unwrap();
        let fonts_path = dir.path().join("fonts");
        std::fs::write(&fonts_path, "font-size = 16\n").unwrap();

        let primary_path = dir.path().join("config");
        std::fs::write(
            &primary_path,
            "font-size = 14\ntheme = dark\nconfig-file = fonts\n",
        )
        .unwrap();

        let tree = read_config_tree(primary_path.to_str().unwrap()).unwrap();
        let map = tree.to_annotated_map();

        // font-size appears in both files
        assert_eq!(map["font-size"].len(), 2);
        assert_eq!(map["font-size"][0].value, "14"); // primary first
        assert_eq!(map["font-size"][1].value, "16"); // include second

        // theme only in primary
        assert_eq!(map["theme"].len(), 1);

        // config-file is also a key-value pair
        assert!(map.contains_key("config-file"));
    }

    #[test]
    fn nested_include_wins_over_its_parent_include() {
        // Ghostty defers config-file loading: a file's includes apply after
        // the file itself, recursively. The docs example: if the primary
        // sets a value and an include overrides it, the include wins -- and
        // the same holds one level deeper.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("grandchild"), "font-size = 20\n").unwrap();
        std::fs::write(
            dir.path().join("child"),
            "config-file = grandchild\nfont-size = 16\n",
        )
        .unwrap();
        let primary = dir.path().join("config");
        std::fs::write(&primary, "config-file = child\nfont-size = 14\n").unwrap();

        let tree = read_config_tree(primary.to_str().unwrap()).unwrap();
        let value = tree.get("font-size").unwrap();
        assert_eq!(value.value, "20", "grandchild loads last and must win");
        assert!(value.source_file.contains("grandchild"));
    }

    #[test]
    fn includes_load_in_queue_order_across_siblings() {
        // Ghostty processes config-file directives as a queue: primary's
        // includes load in declaration order, and includes discovered while
        // loading them append to the end. [primary, a, b, c] here, so c
        // (queued while loading a, after b was already queued) loads last.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("c"), "font-size = 3\n").unwrap();
        std::fs::write(dir.path().join("a"), "config-file = c\nfont-size = 1\n").unwrap();
        std::fs::write(dir.path().join("b"), "font-size = 2\n").unwrap();
        let primary = dir.path().join("config");
        std::fs::write(&primary, "config-file = a\nconfig-file = b\n").unwrap();

        let tree = read_config_tree(primary.to_str().unwrap()).unwrap();
        assert_eq!(tree.get("font-size").unwrap().value, "3");
        let order: Vec<&str> = tree.includes.iter().map(|i| i.directive.as_str()).collect();
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn quoted_include_paths_are_unquoted() {
        // Ghostty supports double-quoted paths (required for paths starting
        // with a literal '?').
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("fonts"), "font-size = 16\n").unwrap();
        let primary = dir.path().join("config");
        std::fs::write(&primary, "config-file = \"fonts\"\n").unwrap();

        let tree = read_config_tree(primary.to_str().unwrap()).unwrap();
        assert_eq!(tree.includes.len(), 1);
        assert_eq!(tree.get("font-size").unwrap().value, "16");
    }

    #[test]
    fn optional_prefix_composes_with_quotes() {
        let dir = tempfile::tempdir().unwrap();
        let primary = dir.path().join("config");
        std::fs::write(
            &primary,
            "config-file = ?\"missing-file\"\nfont-size = 14\n",
        )
        .unwrap();

        let tree = read_config_tree(primary.to_str().unwrap()).unwrap();
        assert_eq!(tree.get("font-size").unwrap().value, "14");
        assert!(tree.includes[0].optional);
        assert!(tree.includes[0].config.is_none());
    }

    #[test]
    fn empty_config_file_directive_is_ignored() {
        // `config-file =` with no value must not resolve to the config
        // directory itself (which fails with IsADirectory and breaks every
        // tool against an otherwise valid config).
        let dir = tempfile::tempdir().unwrap();
        let primary = dir.path().join("config");
        std::fs::write(&primary, "config-file =\nfont-size = 14\n").unwrap();

        let tree = read_config_tree(primary.to_str().unwrap()).unwrap();
        assert_eq!(tree.get("font-size").unwrap().value, "14");
        assert!(tree.includes.is_empty());
    }
}
