use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};

use crate::config::{modules, parser, validator, writer};
use crate::data::options::{self, GhosttyOption};
use crate::search;
use crate::tools::*;

use std::path::Path;

/// Resolve the Ghostty config file path using a fallback chain:
/// 1. GHOSTTY_CONFIG_PATH env var (highest priority)
/// 2. /config/ghostty/config if /config exists (the Docker wrapper mounts
///    the host config directory at /config/ghostty)
/// 3. XDG_CONFIG_HOME/ghostty/config
/// 4. ~/Library/Application Support/com.mitchellh.ghostty/config (macOS)
/// 5. ~/.config/ghostty/config (default)
pub fn resolve_config_path() -> String {
    resolve_config_path_from(std::env::var("GHOSTTY_CONFIG_PATH").ok(), "/config")
}

fn resolve_config_path_from(env_override: Option<String>, docker_base: &str) -> String {
    // 1. Env var override (highest priority)
    if let Some(path) = env_override {
        return path;
    }
    // 2. Docker mount point. install.sh mounts the host config DIRECTORY at
    //    {docker_base}/ghostty, so the config FILE is one level below it.
    if std::path::Path::new(docker_base).exists() {
        return format!("{}/ghostty/config", docker_base);
    }
    // 3. XDG_CONFIG_HOME
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return format!("{}/ghostty/config", xdg);
    }
    // 4. Platform-specific
    if let Ok(home) = std::env::var("HOME") {
        // macOS: ~/Library/Application Support/com.mitchellh.ghostty/config
        let app_support = format!(
            "{}/Library/Application Support/com.mitchellh.ghostty/config",
            home
        );
        if std::path::Path::new(&app_support).exists() {
            return app_support;
        }
        // Default: ~/.config/ghostty/config (works on macOS and Linux)
        return format!("{}/.config/ghostty/config", home);
    }
    format!("{}/ghostty/config", docker_base)
}

#[derive(Clone)]
pub struct GhosttyServer {
    options: Vec<GhosttyOption>,
    config_path: String,
    config_dir: String,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl GhosttyServer {
    pub fn new() -> Self {
        let options = options::load_options();
        let config_path = resolve_config_path();
        let config_dir = Path::new(&config_path)
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string();
        tracing::info!("Loaded {} Ghostty options", options.len());
        tracing::info!("Config path: {}", config_path);
        tracing::info!("Config dir: {}", config_dir);
        Self {
            options,
            config_path,
            config_dir,
            tool_router: Self::tool_router(),
        }
    }

    /// Construct a server bound to an explicit config path (handler tests).
    #[cfg(test)]
    fn with_config_path(config_path: String) -> Self {
        let config_dir = Path::new(&config_path)
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string();
        Self {
            options: options::load_options(),
            config_path,
            config_dir,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Search Ghostty configuration options using fuzzy matching. Returns top matching options with name, short description, default value, and category. Use this to find options by concept (e.g. 'transparent', 'font size', 'padding')."
    )]
    async fn search_config(
        &self,
        params: Parameters<SearchConfigParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let results =
            search::search_options(&self.options, &params.query, params.category.as_deref(), 10);

        if results.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "No options found matching '{}'.\n\nTip: Try broader terms or check available categories with list_categories.",
                params.query
            ))]));
        }

        let mut output = format!(
            "Found {} results for '{}':\n\n",
            results.len(),
            params.query
        );

        for (i, result) in results.iter().enumerate() {
            let opt = &result.option;
            let short_desc: String = opt
                .description
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(100)
                .collect();
            let default = if opt.default_value.is_empty() {
                "(unset)".to_string()
            } else {
                opt.default_value.clone()
            };

            output.push_str(&format!(
                "{}. **{}** [{}]\n   {}\n   Default: `{}`\n\n",
                i + 1,
                opt.name,
                opt.category,
                short_desc,
                default,
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Get complete documentation for a specific Ghostty configuration option by exact name. Returns full description, type, default value, valid values, platform notes, and related options."
    )]
    async fn get_option(
        &self,
        params: Parameters<GetOptionParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let option = self.options.iter().find(|o| o.name == params.name);

        match option {
            Some(opt) => {
                let mut output = format!("# {}\n\n", opt.name);
                output.push_str(&format!("**Category:** {}\n", opt.category));
                output.push_str(&format!("**Type:** {}\n", opt.option_type));

                let default = if opt.default_value.is_empty() {
                    "(unset)".to_string()
                } else {
                    opt.default_value.clone()
                };
                output.push_str(&format!("**Default:** `{}`\n", default));

                if let Some(ref values) = opt.valid_values {
                    output.push_str(&format!(
                        "**Valid values:** {}\n",
                        values
                            .iter()
                            .map(|v| format!("`{}`", v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }

                if let Some(ref platform) = opt.platform {
                    output.push_str(&format!("**Platform:** {}\n", platform.join(", ")));
                }

                output.push_str(&format!("**Reloadable:** {}\n", opt.reloadable));
                output.push_str(&format!("**Repeatable:** {}\n", opt.repeatable));

                output.push_str(&format!("\n## Description\n\n{}\n", opt.description));

                if let Some(ref related) = opt.related_options {
                    output.push_str(&format!("\n## Related Options\n\n{}\n", related.join(", ")));
                }

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            None => {
                let results = search::search_options(&self.options, &params.name, None, 5);
                let suggestions: Vec<String> =
                    results.iter().map(|r| r.option.name.clone()).collect();

                Ok(CallToolResult::error(vec![Content::text(format!(
                    "Option '{}' not found.\n\nDid you mean: {}?\n\nUse search_config to find options by concept.",
                    params.name,
                    suggestions.join(", ")
                ))]))
            }
        }
    }

    #[tool(
        description = "List all Ghostty configuration categories with descriptions and option counts. Use this to browse options by category."
    )]
    async fn list_categories(&self) -> Result<CallToolResult, McpError> {
        let cat_descs = options::category_descriptions();

        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for opt in &self.options {
            *counts.entry(opt.category.clone()).or_insert(0) += 1;
        }

        let mut output = String::from("# Ghostty Configuration Categories\n\n");
        output.push_str(&format!(
            "Total: {} options across {} categories\n\n",
            self.options.len(),
            counts.len()
        ));

        for (name, desc) in &cat_descs {
            if let Some(count) = counts.get(*name) {
                output.push_str(&format!("- **{}** ({} options): {}\n", name, count, desc));
            }
        }

        for (name, count) in &counts {
            if !cat_descs.iter().any(|(n, _)| n == name) {
                output.push_str(&format!("- **{}** ({} options)\n", name, count));
            }
        }

        output.push_str(
            "\nUse `search_config` with a category filter to browse options in a category.",
        );

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Read the current Ghostty configuration. If 'option' is specified, returns just that option's value (with source file if from an include). If omitted, returns all set options grouped by source file. Shows both the current value and default."
    )]
    async fn read_config(
        &self,
        params: Parameters<ReadConfigParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let tree = self.read_tree()?;

        match params.option {
            Some(option_name) => {
                let known = self.options.iter().find(|o| o.name == option_name);
                let is_repeatable = known.is_some_and(|o| o.repeatable);

                let mut output = format!("# {} -- Current Value\n\n", option_name);

                if is_repeatable {
                    let all = tree.get_all(&option_name);
                    if all.is_empty() {
                        output.push_str("**Not set** (using default)\n");
                    } else {
                        output.push_str(&format!("**{} entries set:**\n", all.len()));
                        for av in &all {
                            if av.source_file != self.config_path {
                                output.push_str(&format!(
                                    "- `{}` (from {})\n",
                                    av.value,
                                    Self::short_path(&av.source_file)
                                ));
                            } else {
                                output.push_str(&format!("- `{}`\n", av.value));
                            }
                        }
                    }
                } else {
                    match tree.get(&option_name) {
                        Some(av) => {
                            output.push_str(&format!("**Set to:** `{}`\n", av.value));
                            if av.source_file != self.config_path {
                                output.push_str(&format!(
                                    "**Source:** {}\n",
                                    Self::short_path(&av.source_file)
                                ));
                            }
                        }
                        None => output.push_str("**Not set** (using default)\n"),
                    }
                }

                if let Some(opt) = known {
                    let default = if opt.default_value.is_empty() {
                        "(unset)".to_string()
                    } else {
                        opt.default_value.clone()
                    };
                    output.push_str(&format!("**Default:** `{}`\n", default));
                }

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            None => {
                let annotated_map = tree.to_annotated_map();

                if annotated_map.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(
                        "# Ghostty Configuration\n\nConfig file is empty or not found. All options are using defaults.\n\nUse `write_config` to set options, or `search_config` to find options.",
                    )]));
                }

                // Group by source file for display
                let has_includes = !tree.includes.is_empty();

                if has_includes {
                    let mut output = String::from("# Current Ghostty Configuration\n\n");

                    // Collect options by source file, preserving order
                    let mut by_file: Vec<(String, Vec<(String, String)>)> = Vec::new();

                    // Primary first
                    let primary_map = tree.primary.to_map();
                    if !primary_map.is_empty() {
                        let mut entries = Vec::new();
                        for (key, values) in &primary_map {
                            for v in values {
                                entries.push((key.clone(), v.clone()));
                            }
                        }
                        by_file.push((tree.primary.path.clone(), entries));
                    }

                    // Then each include
                    for inc in &tree.includes {
                        if let Some(ref config) = inc.config {
                            let inc_map = config.to_map();
                            if !inc_map.is_empty() {
                                let mut entries = Vec::new();
                                for (key, values) in &inc_map {
                                    for v in values {
                                        entries.push((key.clone(), v.clone()));
                                    }
                                }
                                by_file.push((config.path.clone(), entries));
                            }
                        }
                    }

                    let total_options: usize = by_file.iter().map(|(_, e)| e.len()).sum();
                    output.push_str(&format!(
                        "{} options set across {} files:\n\n",
                        total_options,
                        by_file.len()
                    ));

                    for (path, entries) in &by_file {
                        output.push_str(&format!("## {}\n\n", Self::short_path(path)));
                        for (key, value) in entries {
                            output.push_str(&format!("{} = {}\n", key, value));
                        }
                        output.push('\n');
                    }

                    Ok(CallToolResult::success(vec![Content::text(output)]))
                } else {
                    // Single file: simple output (backward compatible)
                    let map = tree.primary.to_map();
                    let mut output = format!(
                        "# Current Ghostty Configuration\n\n{} options set:\n\n",
                        map.len()
                    );

                    for (key, values) in &map {
                        if values.len() == 1 {
                            output.push_str(&format!("{} = {}\n", key, values[0]));
                        } else {
                            for v in values {
                                output.push_str(&format!("{} = {}\n", key, v));
                            }
                        }
                    }

                    Ok(CallToolResult::success(vec![Content::text(output)]))
                }
            }
        }
    }

    #[tool(
        description = "Set a Ghostty configuration option. For repeatable options (keybind, palette, etc.), appends a new entry instead of overwriting. For non-repeatable options, updates the existing value or adds it. The value is validated before writing. In modular configs, automatically routes writes to the correct file based on category. Changes take effect on config reload (Cmd+Shift+, on macOS)."
    )]
    async fn write_config(
        &self,
        params: Parameters<WriteConfigParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let known = self.options.iter().find(|o| o.name == params.option);

        let mut warnings = Vec::new();

        if let Some(opt) = known {
            let issues = validator::validate_value(opt, &params.value);
            for issue in &issues {
                match issue.level {
                    validator::IssueLevel::Error => {
                        return Ok(CallToolResult::error(vec![Content::text(format!(
                            "Validation error: {}\n\nThe value was NOT written.",
                            issue.message
                        ))]));
                    }
                    validator::IssueLevel::Warning => {
                        warnings.push(issue.message.clone());
                    }
                }
            }
        } else {
            warnings.push(format!(
                "Unknown option '{}'. It will be written but may not be recognized by Ghostty.",
                params.option
            ));
        }

        // Determine target file using config tree
        let tree = self.read_tree()?;
        let category = known.map(|o| o.category.as_str()).unwrap_or("");
        let target_path = modules::resolve_write_target(&tree, &params.option, category);

        let mut config = parser::read_config(&target_path).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to read config: {}", e).into(),
            data: None,
        })?;

        let is_repeatable = known.is_some_and(|o| o.repeatable);

        let target_label = if target_path != self.config_path {
            format!(" in {}", Self::short_path(&target_path))
        } else {
            String::new()
        };

        let mut output = if is_repeatable {
            // Check for duplicates across all files, not just the target
            let all_existing = tree.get_all(&params.option);
            if all_existing.iter().any(|av| av.value == params.value) {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Duplicate: `{} = {}` already exists in config.",
                    params.option, params.value
                ))]));
            }
            writer::append_option(&mut config, &params.option, &params.value);
            let count = all_existing.len() + 1;
            format!(
                "Appended `{} = {}`{}\n({} total entries)\n",
                params.option, params.value, target_label, count
            )
        } else {
            let previous = config.get(&params.option);
            writer::set_option(&mut config, &params.option, &params.value);
            let mut out = format!(
                "Set `{} = {}`{}\n",
                params.option, params.value, target_label
            );
            if let Some(prev) = previous {
                out.push_str(&format!("Previous value: `{}`\n", prev));
            } else {
                out.push_str("(new option added)\n");
            }
            out
        };

        writer::write_config(&config).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to write config: {}", e).into(),
            data: None,
        })?;

        if !warnings.is_empty() {
            output.push_str(&format!(
                "\nWarnings:\n{}",
                warnings
                    .iter()
                    .map(|w| format!("- {}", w))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        output.push_str("\nReload config in Ghostty (Cmd+Shift+, on macOS) to apply changes.");

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Remove a Ghostty configuration option by commenting it out. For repeatable options (keybind, palette, etc.), provide 'value' to remove a specific entry; omit 'value' to remove all entries. The line is preserved as a comment for reference. In modular configs, operates on the correct file(s)."
    )]
    async fn remove_config(
        &self,
        params: Parameters<RemoveConfigParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let tree = self.read_tree()?;

        // Find all values across the tree
        let all_annotated = tree.get_all(&params.option);
        if all_annotated.is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Option '{}' is not set in any config file.",
                params.option
            ))]));
        }

        // Collect unique files that contain this option
        let mut files_with_option: Vec<String> = Vec::new();
        for av in &all_annotated {
            if !files_with_option.contains(&av.source_file) {
                files_with_option.push(av.source_file.clone());
            }
        }

        let output = if let Some(ref value) = params.value {
            // Remove a specific value -- find the file that contains it
            let mut found = false;
            let mut target_file = String::new();
            for file_path in &files_with_option {
                let mut config = parser::read_config(file_path).map_err(|e| McpError {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: format!("Failed to read config: {}", e).into(),
                    data: None,
                })?;
                if writer::comment_option_value(&mut config, &params.option, value) {
                    writer::write_config(&config).map_err(|e| McpError {
                        code: ErrorCode::INTERNAL_ERROR,
                        message: format!("Failed to write config: {}", e).into(),
                        data: None,
                    })?;
                    found = true;
                    target_file = file_path.clone();
                    break;
                }
            }
            if !found {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "No entry `{} = {}` found in config files.\n\nCurrent values:\n{}",
                    params.option,
                    value,
                    all_annotated
                        .iter()
                        .map(|av| format!(
                            "  {} = {} ({})",
                            params.option,
                            av.value,
                            Self::short_path(&av.source_file)
                        ))
                        .collect::<Vec<_>>()
                        .join("\n")
                ))]));
            }
            // Re-read tree to get accurate count after the write
            let updated_tree = self.read_tree()?;
            let remaining = updated_tree.get_all(&params.option).len();
            let file_label = if target_file != self.config_path {
                format!(" from {}", Self::short_path(&target_file))
            } else {
                String::new()
            };
            format!(
                "Removed `{} = {}`{}\n({} entries remaining)\n",
                params.option, value, file_label, remaining
            )
        } else {
            // Remove all occurrences across all files
            let count = all_annotated.len();
            let mut modified_files = Vec::new();
            for file_path in &files_with_option {
                let mut config = parser::read_config(file_path).map_err(|e| McpError {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: format!("Failed to read config: {}", e).into(),
                    data: None,
                })?;
                if writer::comment_option(&mut config, &params.option) {
                    writer::write_config(&config).map_err(|e| McpError {
                        code: ErrorCode::INTERNAL_ERROR,
                        message: format!("Failed to write config: {}", e).into(),
                        data: None,
                    })?;
                    modified_files.push(file_path.clone());
                }
            }
            let mut out = format!("Removed all {} entries for `{}`\n", count, params.option);
            if modified_files.len() > 1 {
                out.push_str(&format!(
                    "Modified files: {}\n",
                    modified_files
                        .iter()
                        .map(|f| Self::short_path(f))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if let Some(opt) = self.options.iter().find(|o| o.name == params.option) {
                let default = if opt.default_value.is_empty() {
                    "(unset)".to_string()
                } else {
                    opt.default_value.clone()
                };
                out.push_str(&format!("Will revert to default: `{}`\n", default));
            }
            out
        };

        let mut output = output;
        output.push_str("\nReload config in Ghostty to apply changes.");

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Validate the Ghostty configuration. Without parameters, validates the entire config (including all included files) for errors (unknown options, invalid values, type mismatches). With 'option' and 'value', validates a specific value before setting it."
    )]
    async fn validate_config(
        &self,
        params: Parameters<ValidateConfigParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;

        match (params.option, params.value) {
            (Some(option_name), Some(value)) => {
                let known = self.options.iter().find(|o| o.name == option_name);

                match known {
                    Some(opt) => {
                        let issues = validator::validate_value(opt, &value);
                        if issues.is_empty() {
                            Ok(CallToolResult::success(vec![Content::text(format!(
                                "`{} = {}` is valid (type: {}).",
                                option_name, value, opt.option_type
                            ))]))
                        } else {
                            let msg: Vec<String> = issues
                                .iter()
                                .map(|i| format!("[{}] {}", i.level, i.message))
                                .collect();
                            Ok(CallToolResult::error(vec![Content::text(format!(
                                "Validation issues for `{} = {}`:\n\n{}",
                                option_name,
                                value,
                                msg.join("\n")
                            ))]))
                        }
                    }
                    None => Ok(CallToolResult::error(vec![Content::text(format!(
                        "Unknown option '{}'. Use search_config to find the correct option name.",
                        option_name
                    ))])),
                }
            }
            _ => {
                let tree = self.read_tree()?;
                let annotated_map = tree.to_annotated_map();

                if annotated_map.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(
                        "Config is empty. No validation issues.",
                    )]));
                }

                // Build a flat map for validation (merging all files)
                // Skip config-file: it's a meta-directive, not a config option
                let mut flat_map: std::collections::BTreeMap<String, Vec<String>> =
                    std::collections::BTreeMap::new();
                for (key, avs) in &annotated_map {
                    if key == "config-file" {
                        continue;
                    }
                    for av in avs {
                        flat_map
                            .entry(key.clone())
                            .or_default()
                            .push(av.value.clone());
                    }
                }

                let issues = validator::validate_config(&flat_map, &self.options);
                let has_includes = !tree.includes.is_empty();

                if issues.is_empty() {
                    let file_note = if has_includes {
                        let file_count =
                            1 + tree.includes.iter().filter(|i| i.config.is_some()).count();
                        format!(" across {} files", file_count)
                    } else {
                        String::new()
                    };
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "Config is valid. {} options set{}, no issues found.",
                        flat_map.len(),
                        file_note
                    ))]))
                } else {
                    let mut output = format!(
                        "Found {} issues in config ({} options set):\n\n",
                        issues.len(),
                        flat_map.len()
                    );
                    for issue in &issues {
                        // Annotate with source file if modular
                        if has_includes {
                            if let Some(avs) = annotated_map.get(&issue.option) {
                                let sources: Vec<String> = avs
                                    .iter()
                                    .map(|av| Self::short_path(&av.source_file))
                                    .collect::<std::collections::HashSet<_>>()
                                    .into_iter()
                                    .collect();
                                output.push_str(&format!(
                                    "[{}] {} ({}): {}\n",
                                    issue.level,
                                    issue.option,
                                    sources.join(", "),
                                    issue.message
                                ));
                                continue;
                            }
                        }
                        output.push_str(&format!(
                            "[{}] {}: {}\n",
                            issue.level, issue.option, issue.message
                        ));
                    }
                    Ok(CallToolResult::success(vec![Content::text(output)]))
                }
            }
        }
    }

    #[tool(
        description = "List all configuration files in the Ghostty config hierarchy. Shows the primary config file and all included files with their status (loaded/missing/optional), option count, and include source."
    )]
    async fn list_config_files(&self) -> Result<CallToolResult, McpError> {
        let tree = self.read_tree()?;

        let primary_count = tree.primary.to_map().len();
        let mut output = format!(
            "# Ghostty Config Files\n\n**Primary:** {}\n  {} options set\n\n",
            self.config_path, primary_count
        );

        if tree.includes.is_empty() {
            output.push_str("No included config files.\n\nUse `create_config_module` to split your config into modular files (keybinds, fonts, theme).");
        } else {
            output.push_str(&format!(
                "**Includes:** ({} files)\n\n",
                tree.includes.len()
            ));
            for inc in &tree.includes {
                let status = if let Some(ref config) = inc.config {
                    let count = config.to_map().len();
                    format!("loaded, {} options", count)
                } else if inc.optional {
                    "optional, not found".to_string()
                } else {
                    "missing".to_string()
                };

                let optional_marker = if inc.optional { " (optional)" } else { "" };
                let included_from = if inc.included_from != self.config_path {
                    format!(" (included from {})", Self::short_path(&inc.included_from))
                } else {
                    String::new()
                };

                output.push_str(&format!(
                    "- **{}** (`config-file = {}`){}: {}{}\n",
                    Self::short_path(&inc.resolved_path),
                    inc.directive,
                    optional_marker,
                    status,
                    included_from,
                ));
            }
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Create a new modular config file and add it as an include in the primary Ghostty config. Known module names: 'keybinds' (keybind options), 'fonts' (font options), 'theme' (appearance and color options). Custom names are also allowed. After creation, write_config will automatically route matching options to the module file."
    )]
    async fn create_config_module(
        &self,
        params: Parameters<CreateConfigModuleParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;

        // Check if already included
        let tree = self.read_tree()?;
        for inc in &tree.includes {
            let file_name = Path::new(&inc.resolved_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            if file_name == params.name {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Module '{}' is already included in the config.",
                    params.name
                ))]));
            }
        }

        let mut primary = parser::read_config(&self.config_path).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to read config: {}", e).into(),
            data: None,
        })?;

        let module_path = writer::create_module_file(&mut primary, &params.name, &self.config_dir)
            .map_err(|e| McpError {
                code: ErrorCode::INTERNAL_ERROR,
                message: format!("Failed to create module: {}", e).into(),
                data: None,
            })?;

        let categories = modules::categories_for_module(&params.name);
        let mut output = format!(
            "Created config module: {}\n\nFile: {}\nDirective added: `config-file = {}`\n",
            params.name, module_path, params.name
        );

        if !categories.is_empty() {
            output.push_str(&format!(
                "\nOptions in these categories will auto-route to this file:\n{}\n",
                categories
                    .iter()
                    .map(|c| format!("- {}", c))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        output.push_str("\nReload config in Ghostty (Cmd+Shift+, on macOS) to apply changes.");

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }
}

impl GhosttyServer {
    /// Read the full config tree (primary + includes). Re-reads on every call.
    fn read_tree(&self) -> Result<parser::ConfigTree, McpError> {
        parser::read_config_tree(&self.config_path).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to read config tree: {}", e).into(),
            data: None,
        })
    }

    /// Extract a short display name from a file path (just the filename).
    fn short_path(path: &str) -> String {
        Path::new(path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }

    fn build_instructions(&self) -> String {
        // Count options per category
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for opt in &self.options {
            *counts.entry(&opt.category).or_insert(0) += 1;
        }

        // Sort categories by count descending
        let mut sorted_cats: Vec<(&&str, &usize)> = counts.iter().collect();
        sorted_cats.sort_by(|a, b| b.1.cmp(a.1));

        let category_summary: String = sorted_cats
            .iter()
            .map(|(name, count)| format!("{} ({})", name, count))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "Ghostty terminal configuration assistant with {} options across {} categories.\n\
            \n\
            WORKFLOW PATTERNS\n\
            - Discovery: list_categories -> search_config(category=X) -> get_option(name) for full docs\n\
            - Modify: search_config -> get_option (verify type/default) -> write_config -> validate_config\n\
            - Audit: read_config -> validate_config -> fix issues with write_config\n\
            - Remove: read_config(option=X) to check current value -> remove_config to comment it out\n\
            - Remove specific: For repeatable options, use remove_config with 'value' to remove one entry\n\
            - Modular: list_config_files -> create_config_module -> write_config (auto-routes to modules)\n\
            \n\
            SEARCH TIPS\n\
            search_config supports conceptual/natural-language queries, not just exact names.\n\
            - Concepts work: \"transparent\", \"font size\", \"padding\", \"blur\", \"tab style\"\n\
            - Synonym matching is built in (e.g. \"transparent\" finds background-opacity, \"hotkey\" finds keybind)\n\
            - Use the category parameter to narrow results when browsing a topic area\n\
            - get_option returns the full description, type, valid values, and related options\n\
            \n\
            CATEGORIES\n\
            {}\n\
            \n\
            VALIDATION\n\
            - write_config validates automatically before writing and rejects invalid values\n\
            - validate_config without parameters checks the entire config file for errors\n\
            - validate_config with option + value checks a single value before setting it\n\
            - Types: string, boolean, positive integer, float, color (hex like #RRGGBB or named), enum (from fixed set)\n\
            \n\
            CONFIG RELOAD\n\
            After write_config or remove_config, remind the user to reload: Cmd+Shift+, on macOS, or restart Ghostty.",
            self.options.len(),
            counts.len(),
            category_summary,
        )
    }
}

// Since rmcp 1.4 the #[tool_handler] macro defaults to building a fresh
// router per request (Self::tool_router()); point it at the field so the
// router is still built once at construction time.
#[tool_handler(router = self.tool_router)]
impl ServerHandler for GhosttyServer {
    fn get_info(&self) -> ServerInfo {
        // ServerInfo and Implementation are #[non_exhaustive] since rmcp 1.0,
        // so they are built through constructors instead of struct literals.
        // Implementation::new is used rather than from_build_env() because the
        // latter's env! macros expand inside the rmcp crate and would report
        // rmcp's own name/version.
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_server_info(Implementation::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(self.build_instructions())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_config_path_env_override() {
        // GHOSTTY_CONFIG_PATH takes priority over everything
        std::env::set_var("GHOSTTY_CONFIG_PATH", "/tmp/custom-config");
        let path = resolve_config_path();
        std::env::remove_var("GHOSTTY_CONFIG_PATH");
        assert_eq!(path, "/tmp/custom-config");
    }

    #[test]
    fn resolve_config_path_xdg() {
        std::env::remove_var("GHOSTTY_CONFIG_PATH");
        let original_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/xdg-test");
        let path = resolve_config_path();
        // Restore
        match original_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        assert_eq!(path, "/tmp/xdg-test/ghostty/config");
    }

    #[test]
    fn resolve_config_path_returns_string() {
        // Basic sanity: always returns a non-empty path
        let path = resolve_config_path();
        assert!(!path.is_empty());
        assert!(path.contains("ghostty") || path.contains("config"));
    }

    #[test]
    fn resolve_config_path_docker_mount_points_to_config_file() {
        // install.sh mounts the host config DIRECTORY at <base>/ghostty, so
        // the resolved path must be the config FILE inside it. Resolving the
        // directory itself made every config tool fail with IsADirectory
        // (shipped broken since v0.1.0).
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().to_str().unwrap();
        let path = resolve_config_path_from(None, base);
        assert_eq!(path, format!("{base}/ghostty/config"));
    }

    #[test]
    fn resolve_config_path_env_override_beats_docker_mount() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().to_str().unwrap();
        let path = resolve_config_path_from(Some("/tmp/override".to_string()), base);
        assert_eq!(path, "/tmp/override");
    }

    #[test]
    fn server_info_identifies_as_ghostty_mcp() {
        let server = GhosttyServer::new();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "ghostty-mcp");
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn server_new_succeeds() {
        let server = GhosttyServer::new();
        assert!(!server.options.is_empty());
        assert!(!server.config_path.is_empty());
        assert!(!server.config_dir.is_empty());
    }

    #[test]
    fn build_instructions_contains_dynamic_content() {
        let server = GhosttyServer::new();
        let instructions = server.build_instructions();

        // Should contain the actual option count, not a hardcoded approximation
        let count_str = format!("{} options", server.options.len());
        assert!(
            instructions.contains(&count_str),
            "Instructions should contain exact option count '{}', got:\n{}",
            count_str,
            instructions
        );

        // Should contain workflow patterns
        assert!(instructions.contains("WORKFLOW PATTERNS"));
        assert!(instructions.contains("list_categories"));
        assert!(instructions.contains("search_config"));
        assert!(instructions.contains("get_option"));
        assert!(instructions.contains("write_config"));
        assert!(instructions.contains("validate_config"));
        assert!(instructions.contains("remove_config"));

        // Should contain search tips
        assert!(instructions.contains("SEARCH TIPS"));
        assert!(instructions.contains("transparent"));

        // Should contain category summary with actual categories from loaded options
        assert!(instructions.contains("CATEGORIES"));
        assert!(instructions.contains("font"));
        assert!(instructions.contains("window"));

        // Should contain validation info
        assert!(instructions.contains("VALIDATION"));

        // Should contain reload note
        assert!(instructions.contains("CONFIG RELOAD"));

        // Should contain modular workflow
        assert!(instructions.contains("Modular"));
        assert!(instructions.contains("list_config_files"));
        assert!(instructions.contains("create_config_module"));
    }

    // --- MCP tool handler tests -------------------------------------------

    fn text_of(result: &CallToolResult) -> String {
        result
            .content
            .iter()
            .filter_map(|c| c.raw.as_text().map(|t| t.text.clone()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn server_with_config(contents: &str) -> (tempfile::TempDir, GhosttyServer, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config");
        std::fs::write(&path, contents).unwrap();
        let path_str = path.to_string_lossy().to_string();
        let server = GhosttyServer::with_config_path(path_str.clone());
        (dir, server, path_str)
    }

    #[test]
    fn tool_surface_is_stable() {
        // Contract test: renaming or dropping a tool breaks MCP clients.
        let router = GhosttyServer::tool_router();
        let mut names: Vec<String> = router
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "create_config_module",
                "get_option",
                "list_categories",
                "list_config_files",
                "read_config",
                "remove_config",
                "search_config",
                "validate_config",
                "write_config",
            ]
        );
        for tool in GhosttyServer::tool_router().list_all() {
            assert!(
                tool.description.is_some_and(|d| !d.is_empty()),
                "tool {} must have a description",
                tool.name
            );
        }
    }

    #[tokio::test]
    async fn read_config_returns_set_options() {
        let (_dir, server, _) = server_with_config("font-size = 14\ntheme = dark\n");
        let result = server
            .read_config(Parameters(ReadConfigParams { option: None }))
            .await
            .unwrap();
        assert_ne!(result.is_error, Some(true));
        let text = text_of(&result);
        assert!(text.contains("font-size"));
        assert!(text.contains("14"));
        assert!(text.contains("theme"));
    }

    #[tokio::test]
    async fn read_config_single_option_with_default() {
        let (_dir, server, _) = server_with_config("font-size = 14\n");
        let result = server
            .read_config(Parameters(ReadConfigParams {
                option: Some("font-size".to_string()),
            }))
            .await
            .unwrap();
        let text = text_of(&result);
        assert!(text.contains("14"));
    }

    #[tokio::test]
    async fn read_config_missing_file_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent-config");
        let server = GhosttyServer::with_config_path(path.to_string_lossy().to_string());
        let result = server
            .read_config(Parameters(ReadConfigParams { option: None }))
            .await
            .unwrap();
        assert_ne!(
            result.is_error,
            Some(true),
            "a missing config means Ghostty defaults, not a tool error"
        );
    }

    #[tokio::test]
    async fn write_config_updates_existing_option() {
        let (_dir, server, path) = server_with_config("font-size = 14\n");
        let result = server
            .write_config(Parameters(WriteConfigParams {
                option: "font-size".to_string(),
                value: "16".to_string(),
            }))
            .await
            .unwrap();
        assert_ne!(result.is_error, Some(true));
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains("font-size = 16"));
        assert!(!on_disk.contains("font-size = 14"));
    }

    #[tokio::test]
    async fn write_config_appends_repeatable_option() {
        let (_dir, server, path) = server_with_config("keybind = ctrl+a=new_tab\n");
        let result = server
            .write_config(Parameters(WriteConfigParams {
                option: "keybind".to_string(),
                value: "ctrl+b=new_window".to_string(),
            }))
            .await
            .unwrap();
        assert_ne!(result.is_error, Some(true));
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains("ctrl+a=new_tab"));
        assert!(on_disk.contains("ctrl+b=new_window"));
    }

    #[tokio::test]
    async fn write_config_rejects_invalid_enum_value() {
        let (_dir, server, path) = server_with_config("");
        let result = server
            .write_config(Parameters(WriteConfigParams {
                option: "cursor-style".to_string(),
                value: "banana".to_string(),
            }))
            .await
            .unwrap();
        assert_eq!(
            result.is_error,
            Some(true),
            "invalid enum values must be rejected: {}",
            text_of(&result)
        );
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(
            !on_disk.contains("banana"),
            "rejected value must not be written"
        );
    }

    #[tokio::test]
    async fn write_config_unknown_option_writes_with_warning() {
        // Deliberate forward-compatibility: the embedded data may lag behind
        // a newer Ghostty, so unknown options are written with a warning
        // instead of being rejected.
        let (_dir, server, path) = server_with_config("");
        let result = server
            .write_config(Parameters(WriteConfigParams {
                option: "definitely-not-an-option".to_string(),
                value: "x".to_string(),
            }))
            .await
            .unwrap();
        assert_ne!(result.is_error, Some(true));
        assert!(text_of(&result).to_lowercase().contains("unknown option"));
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains("definitely-not-an-option = x"));
    }

    #[tokio::test]
    async fn remove_config_comments_out_option() {
        let (_dir, server, path) = server_with_config("font-size = 14\n");
        let result = server
            .remove_config(Parameters(RemoveConfigParams {
                option: "font-size".to_string(),
                value: None,
            }))
            .await
            .unwrap();
        assert_ne!(result.is_error, Some(true));
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains('#'), "removed option is kept as a comment");
        let config = parser::read_config(&path).unwrap();
        assert_eq!(config.get("font-size"), None);
    }

    #[tokio::test]
    async fn validate_config_accepts_valid_single_value() {
        let (_dir, server, _) = server_with_config("");
        let result = server
            .validate_config(Parameters(ValidateConfigParams {
                option: Some("cursor-style".to_string()),
                value: Some("block".to_string()),
            }))
            .await
            .unwrap();
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn validate_config_flags_invalid_file_entries() {
        let (_dir, server, _) =
            server_with_config("cursor-style = banana\nnot-a-real-option = 1\n");
        let result = server
            .validate_config(Parameters(ValidateConfigParams {
                option: None,
                value: None,
            }))
            .await
            .unwrap();
        let text = text_of(&result);
        assert!(
            text.contains("banana") || text.contains("cursor-style"),
            "invalid enum value must be reported: {text}"
        );
        assert!(
            text.contains("not-a-real-option"),
            "unknown option must be reported: {text}"
        );
    }

    #[tokio::test]
    async fn get_option_unknown_suggests_alternatives() {
        let (_dir, server, _) = server_with_config("");
        let result = server
            .get_option(Parameters(GetOptionParams {
                name: "font-siz".to_string(),
            }))
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(true));
        assert!(text_of(&result).contains("font-size"));
    }

    #[tokio::test]
    async fn search_config_finds_by_concept() {
        let (_dir, server, _) = server_with_config("");
        let result = server
            .search_config(Parameters(SearchConfigParams {
                query: "transparent".to_string(),
                category: None,
            }))
            .await
            .unwrap();
        assert!(text_of(&result).contains("background-opacity"));
    }
}
