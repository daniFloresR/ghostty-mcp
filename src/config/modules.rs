use super::parser::ConfigTree;

/// Module definitions: maps categories to conventional module file names.
/// Only the three most commonly split groupings are mapped.
const MODULE_MAPPINGS: &[(&str, &[&str])] = &[
    ("keybinds", &["keybind"]),
    ("fonts", &["font"]),
    ("theme", &["appearance", "color"]),
];

/// Determine which file a config write should target.
///
/// Priority:
/// 1. If the option already exists in an included file, write there
/// 2. If a module file for this category is included, route there
/// 3. Special: if category is appearance/color AND theme is set, skip theme module
/// 4. Fall back to primary config
pub fn resolve_write_target(tree: &ConfigTree, option_name: &str, option_category: &str) -> String {
    // 1. If the option already exists in an included file, write there
    for inc in &tree.includes {
        if let Some(ref config) = inc.config {
            if config.get(option_name).is_some() {
                return config.path.clone();
            }
        }
    }

    // 2/3. Check if a module file for this category is included
    if let Some(module_name) = module_for_category(option_category) {
        // Special guard: skip theme module if theme is set
        if module_name == "theme" && tree.has_theme_set() {
            return tree.primary.path.clone();
        }

        if let Some(path) = find_module_include(tree, module_name) {
            return path;
        }
    }

    // 4. Fall back to primary
    tree.primary.path.clone()
}

/// Find which module name a category maps to.
fn module_for_category(category: &str) -> Option<&'static str> {
    for (module, categories) in MODULE_MAPPINGS {
        if categories.contains(&category) {
            return Some(module);
        }
    }
    None
}

/// Find an included file that matches a module name.
/// Checks if any include's resolved path has a filename matching the module name.
fn find_module_include(tree: &ConfigTree, module_name: &str) -> Option<String> {
    for inc in &tree.includes {
        if inc.config.is_some() {
            let path = std::path::Path::new(&inc.resolved_path);
            if let Some(file_name) = path.file_name() {
                if file_name.to_string_lossy() == module_name {
                    return Some(inc.resolved_path.clone());
                }
            }
        }
    }
    None
}

/// Get the categories that route to a given module name.
pub fn categories_for_module(module_name: &str) -> Vec<&'static str> {
    for (name, categories) in MODULE_MAPPINGS {
        if *name == module_name {
            return categories.to_vec();
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::parser::read_config_tree;

    #[test]
    fn single_file_always_returns_primary() {
        let dir = tempfile::tempdir().unwrap();
        let primary = dir.path().join("config");
        std::fs::write(&primary, "font-size = 14\n").unwrap();

        let tree = read_config_tree(primary.to_str().unwrap()).unwrap();
        let target = resolve_write_target(&tree, "font-size", "font");
        assert_eq!(target, primary.to_str().unwrap());
    }

    #[test]
    fn option_exists_in_include_routes_there() {
        let dir = tempfile::tempdir().unwrap();
        let custom = dir.path().join("my-custom-keybinds");
        std::fs::write(&custom, "keybind = ctrl+a=new_tab\n").unwrap();

        let primary = dir.path().join("config");
        std::fs::write(&primary, "config-file = my-custom-keybinds\n").unwrap();

        let tree = read_config_tree(primary.to_str().unwrap()).unwrap();
        let target = resolve_write_target(&tree, "keybind", "keybind");
        assert!(target.contains("my-custom-keybinds"));
    }

    #[test]
    fn category_routing_keybind_to_keybinds_module() {
        let dir = tempfile::tempdir().unwrap();
        let keybinds = dir.path().join("keybinds");
        std::fs::write(&keybinds, "# keybinds config\n").unwrap();

        let primary = dir.path().join("config");
        std::fs::write(&primary, "config-file = keybinds\n").unwrap();

        let tree = read_config_tree(primary.to_str().unwrap()).unwrap();
        let target = resolve_write_target(&tree, "keybind", "keybind");
        assert!(target.contains("keybinds"));
    }

    #[test]
    fn theme_guard_with_theme_set() {
        let dir = tempfile::tempdir().unwrap();
        let theme_file = dir.path().join("theme");
        std::fs::write(&theme_file, "background-opacity = 0.9\n").unwrap();

        let primary = dir.path().join("config");
        std::fs::write(&primary, "theme = catppuccin\nconfig-file = theme\n").unwrap();

        let tree = read_config_tree(primary.to_str().unwrap()).unwrap();
        // appearance option should go to primary, not theme module
        let target = resolve_write_target(&tree, "window-theme", "appearance");
        assert_eq!(target, primary.to_str().unwrap());
    }

    #[test]
    fn no_theme_guard_without_theme_set() {
        let dir = tempfile::tempdir().unwrap();
        let theme_file = dir.path().join("theme");
        std::fs::write(&theme_file, "background-opacity = 0.9\n").unwrap();

        let primary = dir.path().join("config");
        std::fs::write(&primary, "config-file = theme\n").unwrap();

        let tree = read_config_tree(primary.to_str().unwrap()).unwrap();
        let target = resolve_write_target(&tree, "window-theme", "appearance");
        assert!(target.contains("theme"));
    }

    #[test]
    fn unmapped_category_goes_to_primary() {
        let dir = tempfile::tempdir().unwrap();
        let keybinds = dir.path().join("keybinds");
        std::fs::write(&keybinds, "# keybinds\n").unwrap();

        let primary = dir.path().join("config");
        std::fs::write(&primary, "config-file = keybinds\n").unwrap();

        let tree = read_config_tree(primary.to_str().unwrap()).unwrap();
        // "shell" category is not mapped to any module
        let target = resolve_write_target(&tree, "command", "shell");
        assert_eq!(target, primary.to_str().unwrap());
    }

    #[test]
    fn user_custom_includes_respected() {
        let dir = tempfile::tempdir().unwrap();
        // User has a custom file with a keybind option
        let custom = dir.path().join("work-keybinds");
        std::fs::write(&custom, "keybind = ctrl+shift+p=command_palette\n").unwrap();

        let primary = dir.path().join("config");
        std::fs::write(&primary, "config-file = work-keybinds\n").unwrap();

        let tree = read_config_tree(primary.to_str().unwrap()).unwrap();
        // Even though there's no "keybinds" module, the option exists in work-keybinds
        let target = resolve_write_target(&tree, "keybind", "keybind");
        assert!(target.contains("work-keybinds"));
    }

    #[test]
    fn categories_for_module_returns_correct_categories() {
        assert_eq!(categories_for_module("keybinds"), vec!["keybind"]);
        assert_eq!(categories_for_module("fonts"), vec!["font"]);
        assert_eq!(categories_for_module("theme"), vec!["appearance", "color"]);
        assert!(categories_for_module("nonexistent").is_empty());
    }
}
