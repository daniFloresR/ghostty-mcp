use super::parser::{ConfigFile, ConfigLine};

/// Update or add an option in the config file.
/// If the option already exists, updates the last occurrence.
/// If it doesn't exist, appends it at the end.
pub fn set_option(config: &mut ConfigFile, key: &str, value: &str) {
    // Find the last occurrence of this key
    let last_idx = config
        .lines
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, line)| match line {
            ConfigLine::KeyValue { key: k, .. } if k == key => Some(i),
            _ => None,
        });

    if let Some(idx) = last_idx {
        // Update existing
        config.lines[idx] = ConfigLine::KeyValue {
            key: key.to_string(),
            value: value.to_string(),
        };
    } else {
        // Append with a blank line separator if the file doesn't end with one
        if !config.lines.is_empty()
            && !matches!(config.lines.last(), Some(ConfigLine::Empty))
        {
            config.lines.push(ConfigLine::Empty);
        }
        config.lines.push(ConfigLine::KeyValue {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
}

/// Comment out an option (preserves the line as a comment instead of deleting).
/// Returns true if the option was found and commented out.
pub fn comment_option(config: &mut ConfigFile, key: &str) -> bool {
    let mut found = false;
    for line in &mut config.lines {
        if let ConfigLine::KeyValue { key: k, value: v } = line {
            if k == key {
                let commented = format!("# {} = {}", k, v);
                *line = ConfigLine::Comment(commented);
                found = true;
            }
        }
    }
    found
}

/// Write the config file back to disk.
pub fn write_config(config: &ConfigFile) -> anyhow::Result<()> {
    let contents = config.to_string();
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&config.path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&config.path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::parser::ConfigFile;

    #[test]
    fn set_option_new_key() {
        let mut config = ConfigFile::parse("font-size = 14\n", "/test");
        set_option(&mut config, "theme", "dark");
        assert_eq!(config.get("theme"), Some("dark".to_string()));
    }

    #[test]
    fn set_option_updates_existing() {
        let mut config = ConfigFile::parse("font-size = 14\n", "/test");
        set_option(&mut config, "font-size", "16");
        assert_eq!(config.get("font-size"), Some("16".to_string()));
        // Should still have only 1 key-value line for font-size
        let count = config.lines.iter().filter(|l| matches!(l, ConfigLine::KeyValue { key, .. } if key == "font-size")).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn set_option_updates_last_of_repeated() {
        let mut config = ConfigFile::parse("font-family = Fira Code\nfont-family = Hack\n", "/test");
        set_option(&mut config, "font-family", "JetBrains Mono");
        // Last occurrence should be updated
        assert_eq!(config.get("font-family"), Some("JetBrains Mono".to_string()));
        // First occurrence should remain unchanged
        let values: Vec<_> = config.to_map()["font-family"].clone();
        assert_eq!(values[0], "Fira Code");
        assert_eq!(values[1], "JetBrains Mono");
    }

    #[test]
    fn set_option_adds_blank_line_separator() {
        let mut config = ConfigFile::parse("font-size = 14\n", "/test");
        set_option(&mut config, "theme", "dark");
        // Should have: KeyValue, Empty, KeyValue
        assert!(matches!(&config.lines[1], ConfigLine::Empty));
    }

    #[test]
    fn comment_option_existing() {
        let mut config = ConfigFile::parse("font-size = 14\ntheme = dark\n", "/test");
        let found = comment_option(&mut config, "font-size");
        assert!(found);
        assert!(config.get("font-size").is_none());
        assert!(matches!(&config.lines[0], ConfigLine::Comment(s) if s.contains("font-size") && s.contains("14")));
    }

    #[test]
    fn comment_option_nonexistent() {
        let mut config = ConfigFile::parse("font-size = 14\n", "/test");
        let found = comment_option(&mut config, "theme");
        assert!(!found);
    }

    #[test]
    fn comment_option_comments_all_occurrences() {
        let mut config = ConfigFile::parse("font-family = Fira Code\nfont-family = Hack\n", "/test");
        let found = comment_option(&mut config, "font-family");
        assert!(found);
        assert!(config.get("font-family").is_none());
        // Both lines should be comments now
        assert!(config.lines.iter().all(|l| matches!(l, ConfigLine::Comment(_))));
    }

    #[test]
    fn write_config_creates_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("config");
        let mut config = ConfigFile::parse("", path.to_str().unwrap());
        set_option(&mut config, "font-size", "14");
        write_config(&config).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("font-size = 14"));
    }
}
