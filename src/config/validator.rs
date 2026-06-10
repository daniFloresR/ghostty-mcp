use crate::data::options::GhosttyOption;

/// A validation issue found in the config.
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub level: IssueLevel,
    pub option: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum IssueLevel {
    Error,
    Warning,
}

impl std::fmt::Display for IssueLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueLevel::Error => write!(f, "ERROR"),
            IssueLevel::Warning => write!(f, "WARNING"),
        }
    }
}

/// Validate a single option value against its type and constraints.
pub fn validate_value(option: &GhosttyOption, value: &str) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if value.is_empty() {
        // Empty values are generally valid (unset)
        return issues;
    }

    match option.option_type.as_str() {
        "boolean" => {
            if !matches!(value, "true" | "false") {
                issues.push(ValidationIssue {
                    level: IssueLevel::Error,
                    option: option.name.clone(),
                    message: format!(
                        "Invalid boolean value '{}'. Expected 'true' or 'false'.",
                        value
                    ),
                });
            }
        }
        "number" => {
            if value.parse::<f64>().is_err() {
                issues.push(ValidationIssue {
                    level: IssueLevel::Error,
                    option: option.name.clone(),
                    message: format!("Invalid number value '{}'.", value),
                });
            }
        }
        "color" => {
            // Validate hex color format or named color
            let v = value.trim_start_matches('#');
            if value.starts_with('#') || value.starts_with("0x") {
                let hex = value.trim_start_matches('#').trim_start_matches("0x");
                if hex.len() != 6 || hex.chars().any(|c| !c.is_ascii_hexdigit()) {
                    issues.push(ValidationIssue {
                        level: IssueLevel::Warning,
                        option: option.name.clone(),
                        message: format!(
                            "Color '{}' may be invalid. Expected format: #RRGGBB or RRGGBB.",
                            value
                        ),
                    });
                }
            } else if v.len() == 6 && v.chars().all(|c| c.is_ascii_hexdigit()) {
                // Valid hex without # prefix
            } else if value.starts_with("cell-") {
                // cell-foreground, cell-background are valid
            } else {
                // Could be a named X11 color — we allow it with a warning
                issues.push(ValidationIssue {
                    level: IssueLevel::Warning,
                    option: option.name.clone(),
                    message: format!(
                        "Color '{}' appears to be a named color. Make sure it's a valid X11 color name.",
                        value
                    ),
                });
            }
        }
        "enum" => {
            if let Some(ref valid) = option.valid_values {
                if !valid.iter().any(|v| v == value) {
                    issues.push(ValidationIssue {
                        level: IssueLevel::Error,
                        option: option.name.clone(),
                        message: format!(
                            "Invalid value '{}'. Valid values are: {}",
                            value,
                            valid.join(", ")
                        ),
                    });
                }
            }
        }
        _ => {
            // String, path, keybind, duration — minimal validation
        }
    }

    issues
}

/// Validate the entire config against known options.
pub fn validate_config(
    config_map: &std::collections::BTreeMap<String, Vec<String>>,
    known_options: &[GhosttyOption],
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    for (key, values) in config_map {
        // Check if option exists
        let option = known_options.iter().find(|o| o.name == *key);

        match option {
            None => {
                issues.push(ValidationIssue {
                    level: IssueLevel::Warning,
                    option: key.clone(),
                    message: format!("Unknown option '{}'. It may be misspelled.", key),
                });
            }
            Some(opt) => {
                // Check for repeated non-repeatable options
                if values.len() > 1 && !opt.repeatable {
                    issues.push(ValidationIssue {
                        level: IssueLevel::Warning,
                        option: key.clone(),
                        message: format!(
                            "Option '{}' is set {} times but is not repeatable. Only the last value will take effect.",
                            key,
                            values.len()
                        ),
                    });
                }

                // Validate each value
                for value in values {
                    issues.extend(validate_value(opt, value));
                }
            }
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::options::GhosttyOption;

    fn make_option(
        name: &str,
        option_type: &str,
        valid_values: Option<Vec<&str>>,
    ) -> GhosttyOption {
        GhosttyOption {
            name: name.to_string(),
            description: String::new(),
            default_value: String::new(),
            option_type: option_type.to_string(),
            valid_values: valid_values.map(|v| v.into_iter().map(String::from).collect()),
            category: "test".to_string(),
            platform: None,
            reloadable: false,
            repeatable: false,
            search_terms: vec![],
            related_options: None,
        }
    }

    #[test]
    fn validate_boolean_valid() {
        let opt = make_option("test", "boolean", None);
        assert!(validate_value(&opt, "true").is_empty());
        assert!(validate_value(&opt, "false").is_empty());
    }

    #[test]
    fn validate_boolean_invalid() {
        let opt = make_option("test", "boolean", None);
        let issues = validate_value(&opt, "yes");
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0].level, IssueLevel::Error));
    }

    #[test]
    fn validate_number_valid() {
        let opt = make_option("test", "number", None);
        assert!(validate_value(&opt, "42").is_empty());
        assert!(validate_value(&opt, "3.14").is_empty());
        assert!(validate_value(&opt, "-1").is_empty());
    }

    #[test]
    fn validate_number_invalid() {
        let opt = make_option("test", "number", None);
        let issues = validate_value(&opt, "abc");
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0].level, IssueLevel::Error));
    }

    #[test]
    fn validate_color_hex_with_hash() {
        let opt = make_option("test", "color", None);
        assert!(validate_value(&opt, "#ff00aa").is_empty());
    }

    #[test]
    fn validate_color_hex_without_hash() {
        let opt = make_option("test", "color", None);
        assert!(validate_value(&opt, "ff00aa").is_empty());
    }

    #[test]
    fn validate_color_invalid_hex() {
        let opt = make_option("test", "color", None);
        let issues = validate_value(&opt, "#xyz");
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0].level, IssueLevel::Warning));
    }

    #[test]
    fn validate_color_cell_prefix() {
        let opt = make_option("test", "color", None);
        assert!(validate_value(&opt, "cell-foreground").is_empty());
    }

    #[test]
    fn validate_color_named_warns() {
        let opt = make_option("test", "color", None);
        let issues = validate_value(&opt, "rebeccapurple");
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0].level, IssueLevel::Warning));
    }

    #[test]
    fn validate_enum_valid() {
        let opt = make_option("test", "enum", Some(vec!["block", "bar", "underline"]));
        assert!(validate_value(&opt, "block").is_empty());
    }

    #[test]
    fn validate_enum_invalid() {
        let opt = make_option("test", "enum", Some(vec!["block", "bar", "underline"]));
        let issues = validate_value(&opt, "circle");
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0].level, IssueLevel::Error));
    }

    #[test]
    fn validate_empty_value_always_ok() {
        let opt = make_option("test", "boolean", None);
        assert!(validate_value(&opt, "").is_empty());
    }

    #[test]
    fn validate_string_type_anything_ok() {
        let opt = make_option("test", "string", None);
        assert!(validate_value(&opt, "literally anything").is_empty());
    }

    #[test]
    fn validate_config_unknown_option() {
        let options = vec![make_option("font-size", "number", None)];
        let mut map = std::collections::BTreeMap::new();
        map.insert("typo-option".to_string(), vec!["value".to_string()]);
        let issues = validate_config(&map, &options);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("Unknown"));
    }

    #[test]
    fn validate_config_repeated_non_repeatable() {
        let options = vec![make_option("font-size", "number", None)];
        let mut map = std::collections::BTreeMap::new();
        map.insert(
            "font-size".to_string(),
            vec!["14".to_string(), "16".to_string()],
        );
        let issues = validate_config(&map, &options);
        assert!(issues.iter().any(|i| i.message.contains("not repeatable")));
    }
}
