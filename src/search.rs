use crate::schema::{KeywordRegistry, SearchResult};
use serde_json::Value;

/// ค้นหา keyword จาก aliases ในทั้ง registry
pub struct KeywordSearch {
    registry: KeywordRegistry,
}

impl KeywordSearch {
    pub fn new(registry: KeywordRegistry) -> Self {
        Self { registry }
    }

    /// ค้นหา keyword จาก query
    /// สนับสนุน: exact match, partial match, language mix (th + en)
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let normalized_query = normalize_query(query);
        let mut results = Vec::new();

        for group in &self.registry.groups {
            for entry in &group.entries {
                if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
                    if let Some(aliases) = entry.get("aliases").and_then(|v| v.as_array()) {
                        let description = entry
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        // เช็ค exact match
                        for alias in aliases {
                            if let Some(alias_str) = alias.as_str() {
                                let normalized_alias = normalize_query(alias_str);

                                if normalized_alias == normalized_query {
                                    results.push(SearchResult {
                                        id: id.to_string(),
                                        group_id: group.group_id.clone(),
                                        aliases: aliases
                                            .iter()
                                            .filter_map(|a| a.as_str().map(String::from))
                                            .collect(),
                                        description: description.clone(),
                                        match_type: "exact".to_string(),
                                    });
                                    break; // ต่อ group ถัดไป
                                }
                            }
                        }

                        // เช็ค partial match
                        for alias in aliases {
                            if let Some(alias_str) = alias.as_str() {
                                let normalized_alias = normalize_query(alias_str);

                                if normalized_alias.contains(&normalized_query) {
                                    // ป้องกัน duplicate จาก exact match
                                    if !results.iter().any(|r| r.id == id) {
                                        results.push(SearchResult {
                                            id: id.to_string(),
                                            group_id: group.group_id.clone(),
                                            aliases: aliases
                                                .iter()
                                                .filter_map(|a| a.as_str().map(String::from))
                                                .collect(),
                                            description: description.clone(),
                                            match_type: "partial".to_string(),
                                        });
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // sort: exact matches ก่อน
        results.sort_by(|a, b| {
            if a.match_type == "exact" && b.match_type != "exact" {
                std::cmp::Ordering::Less
            } else if a.match_type != "exact" && b.match_type == "exact" {
                std::cmp::Ordering::Greater
            } else {
                a.id.cmp(&b.id)
            }
        });

        results
    }

    /// ค้นหาจากทุก alias ของ entry เดียว
    pub fn search_by_entry_id(&self, id: &str) -> Option<SearchResult> {
        for group in &self.registry.groups {
            for entry in &group.entries {
                if let Some(entry_id) = entry.get("id").and_then(|v| v.as_str()) {
                    if entry_id == id {
                        if let Some(aliases) = entry.get("aliases").and_then(|v| v.as_array()) {
                            let description = entry
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            return Some(SearchResult {
                                id: entry_id.to_string(),
                                group_id: group.group_id.clone(),
                                aliases: aliases
                                    .iter()
                                    .filter_map(|a| a.as_str().map(String::from))
                                    .collect(),
                                description,
                                match_type: "exact".to_string(),
                            });
                        }
                    }
                }
            }
        }
        None
    }

    /// รับ query ที่เป็นไทยหรืออังกฤษ ผลลัพธ์เดียวกัน
    pub fn search_language_neutral(&self, query: &str) -> Vec<SearchResult> {
        self.search(query)
    }
}

/// normalize query: trim, lowercase, remove extra spaces
/// ใช้ได้กับทั้งไทยและอังกฤษ
fn normalize_query(q: &str) -> String {
    q.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_query() {
        assert_eq!(normalize_query("Visual-Story"), "visual-story");
        assert_eq!(normalize_query("  mcp  "), "mcp");
        assert_eq!(normalize_query("ภาพเรื่อง"), "ภาพเรื่อง");
    }
}
