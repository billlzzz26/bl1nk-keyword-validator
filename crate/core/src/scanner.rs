use crate::schema::KeywordRegistry;
use crate::ValidatorError;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

/// Usage statistics: keyword_id -> { filename: count }
pub type UsageStats = HashMap<String, HashMap<String, usize>>;

/// Supported file extensions for text scanning
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    // Source code
    "rs", "py", "js", "ts", "go", "java", "cpp", "c", "h", "sh", "rb", "php", "swift", "kt",
    "scala", "r", "lua", "pl", "pm", "ex", "exs", "erl", "hs", "ml", "jl", // Docs
    "md", "txt", "rst", // Config
    "json", "yaml", "yml", "toml", "xml", "ini", "conf", // Web
    "html", "css", "scss", "vue", "svelte",
];

/// Skip binary/unsupported extensions (explicitly unsupported)
pub const UNSUPPORTED_EXTENSIONS: &[&str] = &[
    // Binary
    "exe", "dll", "so", "dylib", "bin", "class", "jar", "war", "pyc", "pyo", // Images
    "png", "jpg", "jpeg", "gif", "svg", "ico", "webp", "bmp", "tiff", "tif",
    // Compressed
    "zip", "tar", "gz", "rar", "7z", "bz2", "xz", // Media
    "mp3", "mp4", "wav", "avi", "mov", "mkv", "flac",
];

/// Count keyword usage in a single file
pub fn count_usage_in_path<P: AsRef<Path>>(
    path: P,
    registry: &KeywordRegistry,
) -> Result<HashMap<String, usize>, ValidatorError> {
    let path = path.as_ref();

    // Check if extension is unsupported (binary)
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if UNSUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
            return Err(ValidatorError::FileIo(format!(
                "Unsupported binary file type: {}",
                ext
            )));
        }
    }

    let content = std::fs::read_to_string(path)
        .map_err(|_| ValidatorError::FileIo(format!("Failed to read: {:?}", path)))?;

    Ok(count_keywords_in_content(&content, registry))
}

/// Count keyword usage in directory (recursive)
pub fn count_usage_in_dir<P: AsRef<Path>>(
    dir: P,
    registry: &KeywordRegistry,
    _extensions: Option<&[&str]>,
) -> Result<UsageStats, ValidatorError> {
    let dir = dir.as_ref();
    let mut stats: UsageStats = HashMap::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            // Skip unsupported extensions
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if UNSUPPORTED_EXTENSIONS.contains(&ext_lower.as_str()) {
                    continue;
                }
            }

            let file_stats = count_usage_in_path(path, registry)?;
            let filename = path.to_string_lossy().to_string();

            for (keyword_id, count) in file_stats {
                stats
                    .entry(keyword_id)
                    .or_default()
                    .insert(filename.clone(), count);
            }
        }
    }

    Ok(stats)
}

/// Aggregate usage stats - returns total count per keyword
pub fn get_total_usage(stats: &UsageStats) -> HashMap<String, usize> {
    stats
        .iter()
        .map(|(keyword_id, file_counts)| (keyword_id.clone(), file_counts.values().sum()))
        .collect()
}

/// Get top N keywords by usage count
pub fn get_top_keywords(stats: &UsageStats, n: usize) -> Vec<(String, usize)> {
    let totals = get_total_usage(stats);
    let mut sorted: Vec<_> = totals.into_iter().collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.1));
    sorted.into_iter().take(n).collect()
}

/// Find related keywords based on co-occurrence in same files
pub fn get_correlation_matrix(stats: &UsageStats) -> HashMap<String, Vec<String>> {
    let mut correlations: HashMap<String, Vec<String>> = HashMap::new();

    // Build correlation: if keyword A and B appear in same file, they are correlated
    for kw1 in stats.keys() {
        let related: Vec<String> = stats
            .iter()
            .filter_map(|(kw2, kw2_files)| {
                if kw1 != kw2 {
                    // Check if both keywords appear in any common file
                    if stats
                        .get(kw1)
                        .map(|kw1_files| kw1_files.iter().any(|(f, _)| kw2_files.contains_key(f)))
                        .unwrap_or(false)
                    {
                        Some(kw2.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if !related.is_empty() {
            correlations.insert(kw1.clone(), related);
        }
    }

    correlations
}

/// Get related keywords for a specific keyword from schema
pub fn find_related(registry: &KeywordRegistry, keyword_id: &str) -> Vec<String> {
    for group in &registry.groups {
        for entry in &group.entries {
            if entry.get("id").and_then(|v| v.as_str()) == Some(keyword_id) {
                if let Some(related) = entry.get("related_to").and_then(|v| v.as_array()) {
                    return related
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                }
            }
        }
    }
    Vec::new()
}

// Internal: count keywords in string content
fn count_keywords_in_content(content: &str, registry: &KeywordRegistry) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for group in &registry.groups {
        for entry in &group.entries {
            let id = entry.get("id").and_then(|v| v.as_str());
            if let Some(id) = id {
                // Count exact ID matches
                let id_count = content.matches(id).count();

                // Count alias matches
                let alias_count = entry
                    .get("aliases")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|a| a.as_str())
                            .map(|alias| content.matches(alias).count())
                            .sum::<usize>()
                    })
                    .unwrap_or(0);

                if id_count + alias_count > 0 {
                    counts.insert(id.to_string(), id_count + alias_count);
                }
            }
        }
    }

    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{KeywordGroup, Metadata};

    fn make_test_registry() -> KeywordRegistry {
        KeywordRegistry {
            version: "1.0.0".to_string(),
            metadata: Metadata {
                last_updated: "2024-01-01".to_string(),
                description: "test".to_string(),
                owner: "test".to_string(),
            },
            groups: vec![KeywordGroup {
                group_id: "g1".to_string(),
                group_name: "Test".to_string(),
                description: "test".to_string(),
                base_fields_schema: Default::default(),
                custom_field_allowed: Default::default(),
                entries: vec![
                    serde_json::json!({
                        "id": "keyword_a",
                        "aliases": ["ka", "k_a"],
                        "description": "test keyword",
                        "type": "constant",
                        "status": "active",
                        "related_to": ["keyword_b", "keyword_c"]
                    }),
                    serde_json::json!({
                        "id": "keyword_b",
                        "aliases": [],
                        "description": "another keyword",
                        "type": "variable",
                        "status": "active"
                    }),
                ],
            }],
            validation: Default::default(),
        }
    }

    #[test]
    fn test_count_keywords_in_content() {
        let registry = make_test_registry();
        let content = "keyword_a ka value keyword_a keyword_b";
        let counts = count_keywords_in_content(content, &registry);

        assert_eq!(counts.get("keyword_a"), Some(&3)); // 2 exact + 1 alias (ka)
        assert_eq!(counts.get("keyword_b"), Some(&1)); // 1 exact, no aliases
    }

    #[test]
    fn test_usage_stats_empty() {
        let registry = make_test_registry();
        let content = "no keywords here";
        let counts = count_keywords_in_content(content, &registry);
        assert!(counts.is_empty());
    }

    #[test]
    fn test_find_related() {
        let registry = make_test_registry();
        let related = find_related(&registry, "keyword_a");
        assert!(related.contains(&"keyword_b".to_string()));
        assert!(related.contains(&"keyword_c".to_string()));
    }

    #[test]
    fn test_get_correlation_matrix() {
        let mut stats: UsageStats = HashMap::new();
        stats
            .entry("a".to_string())
            .or_default()
            .insert("file1.rs".to_string(), 1);
        stats
            .entry("b".to_string())
            .or_default()
            .insert("file1.rs".to_string(), 1);
        stats
            .entry("c".to_string())
            .or_default()
            .insert("file2.rs".to_string(), 1);

        let corr = get_correlation_matrix(&stats);
        assert!(corr.contains_key("a"));
        assert!(corr.get("a").unwrap().contains(&"b".to_string()));
        assert!(!corr.get("a").unwrap().contains(&"c".to_string()));
    }
}
