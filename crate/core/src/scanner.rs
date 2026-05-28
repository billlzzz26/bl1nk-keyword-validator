use crate::schema::KeywordRegistry;
use crate::ValidatorError;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

/// Usage statistics: keyword_id -> { filename: count }
pub type UsageStats = HashMap<String, HashMap<String, usize>>;

/// Count keyword usage in a single file
pub fn count_usage_in_path<P: AsRef<Path>>(
    path: P,
    registry: &KeywordRegistry,
) -> Result<HashMap<String, usize>, ValidatorError> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)
        .map_err(|_| ValidatorError::FileIo(format!("Failed to read: {:?}", path)))?;

    Ok(count_keywords_in_content(&content, registry))
}

/// Count keyword usage in directory (recursive)
pub fn count_usage_in_dir<P: AsRef<Path>>(
    dir: P,
    registry: &KeywordRegistry,
    extensions: Option<&[&str]>,
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
            let ext = path.extension().and_then(|e| e.to_str());
            if let Some(ext) = ext {
                let skip = extensions.map(|exts| !exts.contains(&ext)).unwrap_or(false);
                if skip {
                    continue;
                }
            }

            let file_stats = count_usage_in_path(path, registry)?;
            let filename = path.to_string_lossy().to_string();

            for (keyword_id, count) in file_stats {
                stats
                    .entry(keyword_id)
                    .or_insert_with(HashMap::new)
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
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.into_iter().take(n).collect()
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
                        "status": "active"
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
            language_mapping: None,
            detection_system: None,
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
}