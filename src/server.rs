use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};

use crate::config::{parser, validator, writer};
use crate::data::options::{self, GhosttyOption};
use crate::search;
use crate::tools::*;

/// Resolve the Ghostty config file path using a fallback chain:
/// 1. GHOSTTY_CONFIG_PATH env var (highest priority)
/// 2. /config/ghostty if /config exists (Docker mount point)
/// 3. XDG_CONFIG_HOME/ghostty/config
/// 4. ~/Library/Application Support/com.mitchellh.ghostty/config (macOS)
/// 5. ~/.config/ghostty/config (default)
pub fn resolve_config_path() -> String {
    // 1. Env var override (highest priority)
    if let Ok(path) = std::env::var("GHOSTTY_CONFIG_PATH") {
        return path;
    }
    // 2. Docker mount point (if /config exists, we're in Docker)
    if std::path::Path::new("/config").exists() {
        return "/config/ghostty".to_string();
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
    "/config/ghostty".to_string()
}

#[derive(Clone)]
pub struct GhosttyServer {
    options: Vec<GhosttyOption>,
    config_path: String,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl GhosttyServer {
    pub fn new() -> Self {
        let options = options::load_options();
        let config_path = resolve_config_path();
        tracing::info!("Loaded {} Ghostty options", options.len());
        tracing::info!("Config path: {}", config_path);
        Self {
            options,
            config_path,
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
        description = "Read the current Ghostty configuration. If 'option' is specified, returns just that option's value. If omitted, returns all set options. Shows both the current value and default."
    )]
    async fn read_config(
        &self,
        params: Parameters<ReadConfigParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let config = parser::read_config(&self.config_path).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to read config: {}", e).into(),
            data: None,
        })?;

        match params.option {
            Some(option_name) => {
                let known = self.options.iter().find(|o| o.name == option_name);
                let is_repeatable = known.is_some_and(|o| o.repeatable);

                let mut output = format!("# {} -- Current Value\n\n", option_name);

                if is_repeatable {
                    let all = config.get_all(&option_name);
                    if all.is_empty() {
                        output.push_str("**Not set** (using default)\n");
                    } else {
                        output.push_str(&format!("**{} entries set:**\n", all.len()));
                        for v in &all {
                            output.push_str(&format!("- `{}`\n", v));
                        }
                    }
                } else {
                    match config.get(&option_name) {
                        Some(v) => output.push_str(&format!("**Set to:** `{}`\n", v)),
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
                let map = config.to_map();

                if map.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(
                        "# Ghostty Configuration\n\nConfig file is empty or not found. All options are using defaults.\n\nUse `write_config` to set options, or `search_config` to find options.",
                    )]));
                }

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

    #[tool(
        description = "Set a Ghostty configuration option. For repeatable options (keybind, palette, etc.), appends a new entry instead of overwriting. For non-repeatable options, updates the existing value or adds it. The value is validated before writing. Changes take effect on config reload (Cmd+Shift+, on macOS)."
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

        let mut config = parser::read_config(&self.config_path).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to read config: {}", e).into(),
            data: None,
        })?;

        let is_repeatable = known.is_some_and(|o| o.repeatable);

        let mut output = if is_repeatable {
            let appended = writer::append_option(&mut config, &params.option, &params.value);
            if !appended {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Duplicate: `{} = {}` already exists in config.",
                    params.option, params.value
                ))]));
            }
            let count = config.get_all(&params.option).len();
            format!(
                "Appended `{} = {}`\n({} total entries)\n",
                params.option, params.value, count
            )
        } else {
            let previous = config.get(&params.option);
            writer::set_option(&mut config, &params.option, &params.value);
            let mut out = format!("Set `{} = {}`\n", params.option, params.value);
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
        description = "Remove a Ghostty configuration option by commenting it out. For repeatable options (keybind, palette, etc.), provide 'value' to remove a specific entry; omit 'value' to remove all entries. The line is preserved as a comment for reference."
    )]
    async fn remove_config(
        &self,
        params: Parameters<RemoveConfigParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let mut config = parser::read_config(&self.config_path).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to read config: {}", e).into(),
            data: None,
        })?;

        let all_values = config.get_all(&params.option);
        if all_values.is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Option '{}' is not set in the config file.",
                params.option
            ))]));
        }

        let output = if let Some(ref value) = params.value {
            let found = writer::comment_option_value(&mut config, &params.option, value);
            if !found {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "No entry `{} = {}` found in config file.\n\nCurrent values:\n{}",
                    params.option,
                    value,
                    all_values
                        .iter()
                        .map(|v| format!("  {} = {}", params.option, v))
                        .collect::<Vec<_>>()
                        .join("\n")
                ))]));
            }
            let remaining = config.get_all(&params.option).len();
            format!(
                "Removed `{} = {}`\n({} entries remaining)\n",
                params.option, value, remaining
            )
        } else {
            let count = all_values.len();
            let found = writer::comment_option(&mut config, &params.option);
            if !found {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Option '{}' was not found in config file.",
                    params.option
                ))]));
            }
            let mut out = format!("Removed all {} entries for `{}`\n", count, params.option);
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

        writer::write_config(&config).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to write config: {}", e).into(),
            data: None,
        })?;

        let mut output = output;
        output.push_str("\nReload config in Ghostty to apply changes.");

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Validate the Ghostty configuration. Without parameters, validates the entire config file for errors (unknown options, invalid values, type mismatches). With 'option' and 'value', validates a specific value before setting it."
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
                let config = parser::read_config(&self.config_path).map_err(|e| McpError {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: format!("Failed to read config: {}", e).into(),
                    data: None,
                })?;

                let map = config.to_map();

                if map.is_empty() {
                    return Ok(CallToolResult::success(vec![Content::text(
                        "Config file is empty. No validation issues.",
                    )]));
                }

                let issues = validator::validate_config(&map, &self.options);

                if issues.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "Config is valid. {} options set, no issues found.",
                        map.len()
                    ))]))
                } else {
                    let mut output = format!(
                        "Found {} issues in config ({} options set):\n\n",
                        issues.len(),
                        map.len()
                    );
                    for issue in &issues {
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
}

impl GhosttyServer {
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

#[tool_handler]
impl ServerHandler for GhosttyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(self.build_instructions()),
        }
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
    fn server_new_succeeds() {
        let server = GhosttyServer::new();
        assert!(!server.options.is_empty());
        assert!(!server.config_path.is_empty());
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
    }
}
