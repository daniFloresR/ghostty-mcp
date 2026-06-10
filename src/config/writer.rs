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
        if !config.lines.is_empty() && !matches!(config.lines.last(), Some(ConfigLine::Empty)) {
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

/// Append a new entry for a repeatable option (e.g. keybind, palette).
/// Inserts after the last occurrence of the same key to keep entries grouped.
/// Returns false if an exact duplicate (same key + value) already exists.
pub fn append_option(config: &mut ConfigFile, key: &str, value: &str) -> bool {
    // Reject exact duplicates
    let is_duplicate = config.lines.iter().any(
        |line| matches!(line, ConfigLine::KeyValue { key: k, value: v } if k == key && v == value),
    );
    if is_duplicate {
        return false;
    }

    let new_line = ConfigLine::KeyValue {
        key: key.to_string(),
        value: value.to_string(),
    };

    // Find last occurrence of this key to insert after it
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
        config.lines.insert(idx + 1, new_line);
    } else {
        // No existing entry -- append at end with blank separator
        if !config.lines.is_empty() && !matches!(config.lines.last(), Some(ConfigLine::Empty)) {
            config.lines.push(ConfigLine::Empty);
        }
        config.lines.push(new_line);
    }

    true
}

/// Comment out a specific value of a repeatable option.
/// Unlike `comment_option` which comments ALL occurrences, this only
/// comments the first exact match of key + value.
/// Returns true if a matching entry was found and commented out.
pub fn comment_option_value(config: &mut ConfigFile, key: &str, value: &str) -> bool {
    for line in &mut config.lines {
        if let ConfigLine::KeyValue { key: k, value: v } = line {
            if k == key && v == value {
                let commented = format!("# {} = {}", k, v);
                *line = ConfigLine::Comment(commented);
                return true;
            }
        }
    }
    false
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

/// Create a new module file and add a `config-file` directive to the primary config.
///
/// Returns the path to the newly created module file.
pub fn create_module_file(
    primary: &mut ConfigFile,
    module_name: &str,
    config_dir: &str,
) -> anyhow::Result<String> {
    // Validate module name: no path separators or dots
    if module_name.contains('/')
        || module_name.contains('\\')
        || module_name.contains('.')
        || module_name.is_empty()
    {
        return Err(anyhow::anyhow!(
            "Invalid module name '{}'. Must not contain '/', '\\', or '.'",
            module_name
        ));
    }

    let module_path = format!("{}/{}", config_dir, module_name);

    // Check if file already exists
    if std::path::Path::new(&module_path).exists() {
        return Err(anyhow::anyhow!(
            "Module file already exists: {}",
            module_path
        ));
    }

    // Create the module file with a header comment
    std::fs::write(
        &module_path,
        format!("# Ghostty {} configuration\n", module_name),
    )?;

    // Add config-file directive to primary config
    append_option(primary, "config-file", module_name);

    // Write the updated primary config
    write_config(primary)?;

    Ok(module_path)
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
        let count = config
            .lines
            .iter()
            .filter(|l| matches!(l, ConfigLine::KeyValue { key, .. } if key == "font-size"))
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn set_option_updates_last_of_repeated() {
        let mut config =
            ConfigFile::parse("font-family = Fira Code\nfont-family = Hack\n", "/test");
        set_option(&mut config, "font-family", "JetBrains Mono");
        // Last occurrence should be updated
        assert_eq!(
            config.get("font-family"),
            Some("JetBrains Mono".to_string())
        );
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
        assert!(
            matches!(&config.lines[0], ConfigLine::Comment(s) if s.contains("font-size") && s.contains("14"))
        );
    }

    #[test]
    fn comment_option_nonexistent() {
        let mut config = ConfigFile::parse("font-size = 14\n", "/test");
        let found = comment_option(&mut config, "theme");
        assert!(!found);
    }

    #[test]
    fn comment_option_comments_all_occurrences() {
        let mut config =
            ConfigFile::parse("font-family = Fira Code\nfont-family = Hack\n", "/test");
        let found = comment_option(&mut config, "font-family");
        assert!(found);
        assert!(config.get("font-family").is_none());
        // Both lines should be comments now
        assert!(config
            .lines
            .iter()
            .all(|l| matches!(l, ConfigLine::Comment(_))));
    }

    #[test]
    fn append_option_adds_new_entry() {
        let mut config = ConfigFile::parse("keybind = ctrl+a=new_tab\n", "/test");
        let appended = append_option(&mut config, "keybind", "ctrl+b=new_window");
        assert!(appended);
        let all = config.get_all("keybind");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0], "ctrl+a=new_tab");
        assert_eq!(all[1], "ctrl+b=new_window");
    }

    #[test]
    fn append_option_rejects_exact_duplicate() {
        let mut config = ConfigFile::parse("keybind = ctrl+a=new_tab\n", "/test");
        let appended = append_option(&mut config, "keybind", "ctrl+a=new_tab");
        assert!(!appended);
        assert_eq!(config.get_all("keybind").len(), 1);
    }

    #[test]
    fn append_option_allows_same_trigger_different_action() {
        let mut config = ConfigFile::parse("keybind = ctrl+a=new_tab\n", "/test");
        let appended = append_option(&mut config, "keybind", "ctrl+a=new_window");
        assert!(appended);
        assert_eq!(config.get_all("keybind").len(), 2);
    }

    #[test]
    fn append_option_groups_after_last_occurrence() {
        let mut config = ConfigFile::parse(
            "font-size = 14\nkeybind = ctrl+a=new_tab\ntheme = dark\n",
            "/test",
        );
        append_option(&mut config, "keybind", "ctrl+b=new_window");
        let output = config.to_string();
        let lines: Vec<&str> = output.lines().collect();
        let positions: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.starts_with("keybind"))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[1], positions[0] + 1); // adjacent
    }

    #[test]
    fn append_option_first_entry() {
        let mut config = ConfigFile::parse("font-size = 14\n", "/test");
        let appended = append_option(&mut config, "keybind", "ctrl+a=new_tab");
        assert!(appended);
        assert_eq!(config.get("keybind"), Some("ctrl+a=new_tab".to_string()));
    }

    #[test]
    fn comment_option_value_removes_specific_entry() {
        let mut config = ConfigFile::parse(
            "keybind = ctrl+a=new_tab\nkeybind = ctrl+b=new_window\n",
            "/test",
        );
        let found = comment_option_value(&mut config, "keybind", "ctrl+a=new_tab");
        assert!(found);
        let all = config.get_all("keybind");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], "ctrl+b=new_window");
    }

    #[test]
    fn comment_option_value_nonexistent() {
        let mut config = ConfigFile::parse("keybind = ctrl+a=new_tab\n", "/test");
        let found = comment_option_value(&mut config, "keybind", "ctrl+z=quit");
        assert!(!found);
        assert_eq!(config.get_all("keybind").len(), 1);
    }

    #[test]
    fn comment_option_value_preserves_as_comment() {
        let mut config = ConfigFile::parse(
            "keybind = ctrl+a=new_tab\nkeybind = ctrl+b=new_window\n",
            "/test",
        );
        comment_option_value(&mut config, "keybind", "ctrl+a=new_tab");
        assert!(config
            .lines
            .iter()
            .any(|l| matches!(l, ConfigLine::Comment(s) if s.contains("ctrl+a=new_tab"))));
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

    #[test]
    fn create_module_file_creates_file_with_header() {
        let dir = tempfile::tempdir().unwrap();
        let primary_path = dir.path().join("config");
        std::fs::write(&primary_path, "font-size = 14\n").unwrap();
        let mut primary = ConfigFile::parse(
            &std::fs::read_to_string(&primary_path).unwrap(),
            primary_path.to_str().unwrap(),
        );

        let result = create_module_file(&mut primary, "keybinds", dir.path().to_str().unwrap());
        assert!(result.is_ok());

        let module_path = result.unwrap();
        let contents = std::fs::read_to_string(&module_path).unwrap();
        assert!(contents.contains("# Ghostty keybinds configuration"));
    }

    #[test]
    fn create_module_file_adds_directive_to_primary() {
        let dir = tempfile::tempdir().unwrap();
        let primary_path = dir.path().join("config");
        std::fs::write(&primary_path, "font-size = 14\n").unwrap();
        let mut primary = ConfigFile::parse(
            &std::fs::read_to_string(&primary_path).unwrap(),
            primary_path.to_str().unwrap(),
        );

        create_module_file(&mut primary, "keybinds", dir.path().to_str().unwrap()).unwrap();

        // Re-read the primary config from disk
        let updated = std::fs::read_to_string(&primary_path).unwrap();
        assert!(updated.contains("config-file = keybinds"));
    }

    #[test]
    fn create_module_file_rejects_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        let primary_path = dir.path().join("config");
        std::fs::write(&primary_path, "font-size = 14\n").unwrap();

        // Create the module file first (simulate it already existing)
        let module_path = dir.path().join("keybinds");
        std::fs::write(&module_path, "# existing\n").unwrap();

        let mut primary = ConfigFile::parse(
            &std::fs::read_to_string(&primary_path).unwrap(),
            primary_path.to_str().unwrap(),
        );

        let result = create_module_file(&mut primary, "keybinds", dir.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn create_module_file_rejects_invalid_names() {
        let dir = tempfile::tempdir().unwrap();
        let primary_path = dir.path().join("config");
        std::fs::write(&primary_path, "font-size = 14\n").unwrap();
        let mut primary = ConfigFile::parse(
            &std::fs::read_to_string(&primary_path).unwrap(),
            primary_path.to_str().unwrap(),
        );

        // Slashes
        assert!(
            create_module_file(&mut primary, "sub/file", dir.path().to_str().unwrap()).is_err()
        );

        // Backslashes
        assert!(
            create_module_file(&mut primary, "sub\\file", dir.path().to_str().unwrap()).is_err()
        );

        // Dots
        assert!(
            create_module_file(&mut primary, "file.conf", dir.path().to_str().unwrap()).is_err()
        );

        // Empty
        assert!(create_module_file(&mut primary, "", dir.path().to_str().unwrap()).is_err());
    }
}
