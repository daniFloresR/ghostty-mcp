use serde::{Deserialize, Serialize};

/// A single Ghostty configuration option with full documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhosttyOption {
    pub name: String,
    pub description: String,
    pub default_value: String,
    pub option_type: String,
    pub valid_values: Option<Vec<String>>,
    pub category: String,
    pub platform: Option<Vec<String>>,
    pub reloadable: bool,
    pub repeatable: bool,
    pub search_terms: Vec<String>,
    pub related_options: Option<Vec<String>>,
}

const OPTIONS_JSON: &str = include_str!("../../data/ghostty-options.json");

/// Load all options from the embedded JSON (parsed at compile time via include_str!).
pub fn load_options() -> Vec<GhosttyOption> {
    serde_json::from_str(OPTIONS_JSON).expect("Failed to parse embedded ghostty-options.json")
}

/// Get category descriptions.
pub fn category_descriptions() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "font",
            "Font family, size, style, metrics, and rendering options",
        ),
        (
            "window",
            "Window appearance, padding, decorations, tabs, and splits",
        ),
        (
            "appearance",
            "Theme, colors, opacity, contrast, and visual effects",
        ),
        ("cursor", "Cursor style, color, blinking, and behavior"),
        (
            "mouse",
            "Mouse behavior, hide-while-typing, scroll settings",
        ),
        ("color", "Color palette, bold/faint colors"),
        ("selection", "Text selection behavior and colors"),
        ("clipboard", "Clipboard read/write permissions and behavior"),
        ("keybind", "Keyboard shortcuts and key bindings"),
        (
            "shell",
            "Shell command, working directory, and shell behavior",
        ),
        (
            "shell-integration",
            "Shell integration features and prompt detection",
        ),
        ("scrollback", "Scrollback buffer size and behavior"),
        ("link", "Clickable URL detection and behavior"),
        (
            "macos",
            "macOS-specific options (titlebar, option-as-alt, etc.)",
        ),
        ("linux", "Linux-specific options"),
        ("gtk", "GTK-specific options (Linux)"),
        ("x11", "X11-specific options (Linux)"),
        ("quick-terminal", "Quick terminal dropdown settings"),
        ("shader", "Custom GLSL shader effects"),
        (
            "compatibility",
            "Terminal compatibility settings (TERM, OSC, etc.)",
        ),
        ("image", "Image rendering and storage settings"),
        ("general", "General application settings"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_options_succeeds() {
        let options = load_options();
        assert!(!options.is_empty());
    }

    #[test]
    fn options_have_required_fields() {
        let options = load_options();
        for opt in &options {
            assert!(!opt.name.is_empty(), "Option name should not be empty");
            assert!(
                !opt.category.is_empty(),
                "Option {} should have a category",
                opt.name
            );
            assert!(
                !opt.option_type.is_empty(),
                "Option {} should have a type",
                opt.name
            );
        }
    }

    #[test]
    fn at_least_150_options_embedded() {
        // The data pipeline guarantees >= 150 options; pin it at compile-test
        // time too so a broken regeneration cannot ship.
        assert!(load_options().len() >= 150);
    }

    #[test]
    fn option_names_are_unique() {
        let options = load_options();
        let mut names: Vec<&str> = options.iter().map(|o| o.name.as_str()).collect();
        names.sort();
        let before = names.len();
        names.dedup();
        assert_eq!(
            before,
            names.len(),
            "duplicate option names in embedded data"
        );
    }

    #[test]
    fn no_empty_descriptions() {
        // Grouped options (font-family-bold etc.) inherit their group's
        // description in the parser; nothing should ship undocumented.
        let options = load_options();
        let empty: Vec<&str> = options
            .iter()
            .filter(|o| o.description.is_empty())
            .map(|o| o.name.as_str())
            .collect();
        assert!(
            empty.is_empty(),
            "options with empty descriptions: {empty:?}"
        );
    }

    #[test]
    fn enum_options_exist_and_carry_valid_values() {
        // Regression net for the dead enum-inference bug: the parser shipped
        // 0 enums for months and the validator's enum branch never ran.
        let options = load_options();
        let enums: Vec<_> = options.iter().filter(|o| o.option_type == "enum").collect();
        assert!(
            enums.len() >= 20,
            "expected at least 20 enum options, found {}",
            enums.len()
        );
        for opt in &enums {
            let values = opt.valid_values.as_ref();
            assert!(
                values.is_some_and(|v| v.len() >= 2),
                "enum option {} must have at least 2 valid_values",
                opt.name
            );
        }
    }

    #[test]
    fn cursor_style_enum_canary() {
        // Concrete end-to-end canary for the parser -> data -> validator
        // contract on a known stable option.
        let options = load_options();
        let cursor_style = options
            .iter()
            .find(|o| o.name == "cursor-style")
            .expect("cursor-style option must exist");
        assert_eq!(cursor_style.option_type, "enum");
        let values = cursor_style.valid_values.as_ref().unwrap();
        for expected in ["block", "bar", "underline", "block_hollow"] {
            assert!(
                values.iter().any(|v| v == expected),
                "cursor-style must accept {expected}"
            );
        }
    }

    #[test]
    fn category_descriptions_cover_all_categories() {
        let options = load_options();
        let descriptions = category_descriptions();
        let desc_names: Vec<&str> = descriptions.iter().map(|(n, _)| *n).collect();

        let mut categories: Vec<String> = options.iter().map(|o| o.category.clone()).collect();
        categories.sort();
        categories.dedup();

        for cat in &categories {
            assert!(
                desc_names.contains(&cat.as_str()),
                "Category '{}' missing from category_descriptions()",
                cat
            );
        }
    }
}
