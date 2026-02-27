use nucleo_matcher::{
    Config, Matcher,
    pattern::{CaseMatching, Normalization, Pattern},
};

use crate::data::options::GhosttyOption;

/// Search result with score for ranking.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub option: GhosttyOption,
    pub score: u32,
}

/// Perform fuzzy search over Ghostty options.
///
/// Searches across name, search_terms, and description (weighted by field).
/// Returns top `limit` results sorted by score descending.
pub fn search_options(
    options: &[GhosttyOption],
    query: &str,
    category: Option<&str>,
    limit: usize,
) -> Vec<SearchResult> {
    if query.is_empty() {
        // No query: return all options (optionally filtered by category)
        let mut results: Vec<SearchResult> = options
            .iter()
            .filter(|o| category.is_none_or(|c| o.category == c))
            .map(|o| SearchResult {
                option: o.clone(),
                score: 0,
            })
            .collect();
        results.truncate(limit);
        return results;
    }

    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

    let mut scored: Vec<SearchResult> = options
        .iter()
        .filter(|o| category.is_none_or(|c| o.category == c))
        .filter_map(|option| {
            // Build searchable text: name has highest weight, then search_terms, then description
            // We achieve weighting by repeating the name
            let search_text = format!(
                "{name} {name} {name} {terms} {desc_short}",
                name = option.name,
                terms = option.search_terms.join(" "),
                desc_short = option.description.lines().next().unwrap_or(""),
            );

            let haystack: Vec<char> = search_text.chars().collect();
            let haystack_str = nucleo_matcher::Utf32Str::Unicode(&haystack);

            pattern.score(haystack_str, &mut matcher).map(|score| SearchResult {
                option: option.clone(),
                score,
            })
        })
        .collect();

    scored.sort_by(|a, b| b.score.cmp(&a.score));
    scored.truncate(limit);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::options::GhosttyOption;

    fn make_options() -> Vec<GhosttyOption> {
        vec![
            GhosttyOption {
                name: "font-size".to_string(),
                description: "The size of the font in points.".to_string(),
                default_value: "13".to_string(),
                option_type: "number".to_string(),
                valid_values: None,
                category: "font".to_string(),
                platform: None,
                reloadable: true,
                repeatable: false,
                search_terms: vec!["size".to_string(), "points".to_string()],
                related_options: None,
            },
            GhosttyOption {
                name: "background-opacity".to_string(),
                description: "The opacity of the background.".to_string(),
                default_value: "1.0".to_string(),
                option_type: "number".to_string(),
                valid_values: None,
                category: "appearance".to_string(),
                platform: None,
                reloadable: true,
                repeatable: false,
                search_terms: vec!["transparent".to_string(), "opacity".to_string()],
                related_options: None,
            },
            GhosttyOption {
                name: "font-family".to_string(),
                description: "The font family to use.".to_string(),
                default_value: "".to_string(),
                option_type: "string".to_string(),
                valid_values: None,
                category: "font".to_string(),
                platform: None,
                reloadable: true,
                repeatable: true,
                search_terms: vec!["family".to_string(), "typeface".to_string()],
                related_options: None,
            },
        ]
    }

    #[test]
    fn search_empty_query_returns_all() {
        let options = make_options();
        let results = search_options(&options, "", None, 100);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn search_empty_query_with_category_filter() {
        let options = make_options();
        let results = search_options(&options, "", Some("font"), 100);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.option.category == "font"));
    }

    #[test]
    fn search_exact_name_match() {
        let options = make_options();
        let results = search_options(&options, "font-size", None, 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].option.name, "font-size");
    }

    #[test]
    fn search_fuzzy_match() {
        let options = make_options();
        let results = search_options(&options, "transparent", None, 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].option.name, "background-opacity");
    }

    #[test]
    fn search_respects_limit() {
        let options = make_options();
        let results = search_options(&options, "", None, 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_category_filter_with_query() {
        let options = make_options();
        let results = search_options(&options, "font", Some("appearance"), 10);
        // Should not return font options since we filter by appearance
        assert!(results.iter().all(|r| r.option.category == "appearance"));
    }

    #[test]
    fn search_no_match_returns_empty() {
        let options = make_options();
        let results = search_options(&options, "zzzznonexistent", None, 10);
        assert!(results.is_empty());
    }
}
