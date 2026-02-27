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
        ("font", "Font family, size, style, metrics, and rendering options"),
        ("window", "Window appearance, padding, decorations, tabs, and splits"),
        ("appearance", "Theme, colors, opacity, contrast, and visual effects"),
        ("cursor", "Cursor style, color, blinking, and behavior"),
        ("mouse", "Mouse behavior, hide-while-typing, scroll settings"),
        ("color", "Color palette, bold/faint colors"),
        ("selection", "Text selection behavior and colors"),
        ("clipboard", "Clipboard read/write permissions and behavior"),
        ("keybind", "Keyboard shortcuts and key bindings"),
        ("shell", "Shell command, working directory, and shell behavior"),
        ("shell-integration", "Shell integration features and prompt detection"),
        ("scrollback", "Scrollback buffer size and behavior"),
        ("link", "Clickable URL detection and behavior"),
        ("macos", "macOS-specific options (titlebar, option-as-alt, etc.)"),
        ("linux", "Linux-specific options"),
        ("gtk", "GTK-specific options (Linux)"),
        ("x11", "X11-specific options (Linux)"),
        ("quick-terminal", "Quick terminal dropdown settings"),
        ("shader", "Custom GLSL shader effects"),
        ("compatibility", "Terminal compatibility settings (TERM, OSC, etc.)"),
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
            assert!(!opt.category.is_empty(), "Option {} should have a category", opt.name);
            assert!(!opt.option_type.is_empty(), "Option {} should have a type", opt.name);
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
