use crate::error::ValidationError;
use crate::schema::KeywordRegistry;
use serde_json::Value;
use std::collections::HashMap;

pub struct Validator {
    registry: KeywordRegistry,
}

impl Validator {
    pub fn new(registry: KeywordRegistry) -> Self {
        Self { registry }
    }

    /// ตรวจสอบ duplicate aliases ในกลุ่มเดียวกันหรือข้ามกลุ่ม
    /// ถ้า editing_entry_id บางตัว จะข้ามการตรวจสอบ alias ของ entry นั้น
    pub fn check_duplicate_aliases(
        &self,
        group_id: &str,
        editing_entry_id: Option<&str>,
        new_aliases: &[String],
    ) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut alias_map: HashMap<String, (String, String)> = HashMap::new(); // alias -> (group_id, entry_id)

        // สร้าง map ของ aliases ที่มีอยู่ทั้งหมด
        for group in &self.registry.groups {
            for (_entry_idx, entry) in group.entries.iter().enumerate() {
                if let Some(entry_id) = entry.get("id").and_then(|v| v.as_str()) {
                    // ข้าม entry ที่กำลังแก้ไขอยู่
                    if let Some(editing_id) = editing_entry_id {
                        if entry_id == editing_id && group.group_id == group_id {
                            continue;
                        }
                    }

                    if let Some(aliases) = entry.get("aliases").and_then(|v| v.as_array()) {
                        for alias in aliases {
                            if let Some(alias_str) = alias.as_str() {
                                alias_map.insert(
                                    alias_str.to_lowercase(),
                                    (group.group_id.clone(), entry_id.to_string()),
                                );
                            }
                        }
                    }
                }
            }
        }

        // ตรวจสอบ new_aliases ว่ามีซ้ำหรือไม่
        for new_alias in new_aliases {
            let lower_alias = new_alias.to_lowercase();
            if let Some((existing_group, existing_entry)) = alias_map.get(&lower_alias) {
                errors.push(ValidationError {
                    code: "DUPLICATE_ALIAS".to_string(),
                    message: format!(
                        "Alias '{}' already exists in group '{}' entry '{}'",
                        new_alias, existing_group, existing_entry
                    ),
                    field: Some("aliases".to_string()),
                });
            }
        }

        errors
    }

    /// validate entry เดียวตาม group_id
    pub fn validate_entry(
        &self,
        group_id: &str,
        entry: &Value,
    ) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // หา group
        let group = self
            .registry
            .groups
            .iter()
            .find(|g| g.group_id == group_id);

        let group = match group {
            Some(g) => g,
            None => {
                return Err(vec![ValidationError {
                    code: "GROUP_NOT_FOUND".to_string(),
                    message: format!("Group '{}' not found", group_id),
                    field: None,
                }]);
            }
        };

        // ตรวจสอบ required base fields
        for (field_name, field_schema) in &group.base_fields_schema {
            if field_schema.required.unwrap_or(false) {
                if entry.get(field_name).is_none() {
                    errors.push(ValidationError {
                        code: "MISSING_REQUIRED_FIELD".to_string(),
                        message: format!("Missing required field '{}'", field_name),
                        field: Some(field_name.clone()),
                    });
                }
            }
        }

        // ตรวจสอบ type ของแต่ละ field
        for (field_name, field_schema) in &group.base_fields_schema {
            if let Some(field_value) = entry.get(field_name) {
                // ตรวจสอบ type
                match field_schema.field_type.as_str() {
                    "string" => {
                        if !field_value.is_string() {
                            errors.push(ValidationError {
                                code: "INVALID_TYPE".to_string(),
                                message: format!(
                                    "Field '{}' expected type 'string' but got {:?}",
                                    field_name,
                                    field_value
                                ),
                                field: Some(field_name.clone()),
                            });
                        }
                    }
                    "array" => {
                        if let Some(arr) = field_value.as_array() {
                            // ตรวจสอบ array item type
                            if let Some(item_type) = &field_schema.item_type {
                                for (idx, item) in arr.iter().enumerate() {
                                    match item_type.as_str() {
                                        "string" => {
                                            if !item.is_string() {
                                                errors.push(ValidationError {
                                                    code: "INVALID_TYPE".to_string(),
                                                    message: format!(
                                                        "Array item {} in field '{}' expected type 'string'",
                                                        idx, field_name
                                                    ),
                                                    field: Some(field_name.clone()),
                                                });
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            // ตรวจสอบ alias length
                            if field_name == "aliases" {
                                let rules = &self.registry.validation.rules;
                                for item in arr {
                                    if let Some(alias_str) = item.as_str() {
                                        if alias_str.len() < rules.alias_min_length {
                                            errors.push(ValidationError {
                                                code: "ALIAS_TOO_SHORT".to_string(),
                                                message: format!(
                                                    "Alias '{}' is too short (min {} chars)",
                                                    alias_str, rules.alias_min_length
                                                ),
                                                field: Some("aliases".to_string()),
                                            });
                                        }
                                        if alias_str.len() > rules.alias_max_length {
                                            errors.push(ValidationError {
                                                code: "ALIAS_TOO_LONG".to_string(),
                                                message: format!(
                                                    "Alias '{}' is too long (max {} chars)",
                                                    alias_str, rules.alias_max_length
                                                ),
                                                field: Some("aliases".to_string()),
                                            });
                                        }
                                    }
                                }
                            }
                        } else {
                            errors.push(ValidationError {
                                code: "INVALID_TYPE".to_string(),
                                message: format!(
                                    "Field '{}' expected type 'array' but got {:?}",
                                    field_name, field_value
                                ),
                                field: Some(field_name.clone()),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// validate ทั้ง registry
    pub fn validate_registry(&self) -> Result<(), Vec<ValidationError>> {
        let mut all_errors = Vec::new();

        // 1. เก็บ ID ทั้งหมดเพื่อเช็ค Broken Links (relatedIds)
        let mut all_valid_ids = std::collections::HashSet::new();
        for group in &self.registry.groups {
            for entry in &group.entries {
                if let Some(entry_id) = entry.get("id").and_then(|v| v.as_str()) {
                    all_valid_ids.insert(entry_id.to_string());
                }
            }
        }

        for group in &self.registry.groups {
            for (_entry_idx, entry) in group.entries.iter().enumerate() {
                if let Some(entry_id) = entry.get("id").and_then(|v| v.as_str()) {
                    match self.validate_entry(&group.group_id, entry) {
                        Ok(_) => {}
                        Err(mut errors) => {
                            all_errors.append(&mut errors);
                        }
                    }

                    // 2. ตรวจสอบ duplicate aliases
                    if let Some(aliases) = entry.get("aliases").and_then(|v| v.as_array()) {
                        let alias_strings: Vec<String> = aliases
                            .iter()
                            .filter_map(|a| a.as_str().map(|s| s.to_string()))
                            .collect();

                        let dup_errors = self.check_duplicate_aliases(
                            &group.group_id,
                            Some(entry_id),
                            &alias_strings,
                        );
                        all_errors.extend(dup_errors);
                    }

                    // 3. ตรวจสอบ Broken Links (relatedIds)
                    if let Some(related_ids) = entry.get("relatedIds").and_then(|v| v.as_array()) {
                        for (idx, rel_id_val) in related_ids.iter().enumerate() {
                            if let Some(rel_id) = rel_id_val.as_str() {
                                if !all_valid_ids.contains(rel_id) {
                                    all_errors.push(ValidationError {
                                        code: "BROKEN_RELATIONSHIP".to_string(),
                                        message: format!(
                                            "Entry '{}' references non-existent ID '{}' in relatedIds[{}]",
                                            entry_id, rel_id, idx
                                        ),
                                        field: Some("relatedIds".to_string()),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        if all_errors.is_empty() {
            Ok(())
        } else {
            Err(all_errors)
        }
    }

    /// อ้างอิง registry
    pub fn registry(&self) -> &KeywordRegistry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        CustomFieldConfig, FieldSchema, KeywordGroup, KeywordRegistry, Metadata, ValidationConfig,
        ValidationRules,
    };
    use serde_json::json;
    use std::collections::HashMap;

    fn make_test_registry() -> KeywordRegistry {
        KeywordRegistry {
            version: "1.0.0".to_string(),
            metadata: Metadata {
                last_updated: "2026-04-04T00:00:00Z".to_string(),
                description: "Test registry".to_string(),
                owner: "test".to_string(),
            },
            groups: vec![
                KeywordGroup {
                    group_id: "projects".to_string(),
                    group_name: "Projects".to_string(),
                    description: "Test projects".to_string(),
                    base_fields_schema: {
                        let mut map = HashMap::new();
                        map.insert(
                            "id".to_string(),
                            FieldSchema {
                                field_type: "string".to_string(),
                                item_type: None,
                                pattern: None,
                                values: None,
                                required: Some(true),
                                max_length: None,
                                description: "Project ID".to_string(),
                            },
                        );
                        map.insert(
                            "aliases".to_string(),
                            FieldSchema {
                                field_type: "array".to_string(),
                                item_type: Some("string".to_string()),
                                pattern: None,
                                values: None,
                                required: Some(true),
                                max_length: None,
                                description: "Search aliases".to_string(),
                            },
                        );
                        map
                    },
                    custom_field_allowed: CustomFieldConfig {
                        enabled: false,
                        max_one: false,
                        description: "".to_string(),
                        examples: None,
                    },
                    entries: vec![
                        json!({
                            "id": "proj-alpha",
                            "aliases": ["alpha", "project-alpha", "โปรเจกต์อัลฟา"]
                        }),
                        json!({
                            "id": "proj-beta",
                            "aliases": ["beta", "project-beta"]
                        }),
                    ],
                },
                KeywordGroup {
                    group_id: "skills".to_string(),
                    group_name: "Skills".to_string(),
                    description: "Test skills".to_string(),
                    base_fields_schema: {
                        let mut map = HashMap::new();
                        map.insert(
                            "id".to_string(),
                            FieldSchema {
                                field_type: "string".to_string(),
                                item_type: None,
                                pattern: None,
                                values: None,
                                required: Some(true),
                                max_length: None,
                                description: "Skill ID".to_string(),
                            },
                        );
                        map.insert(
                            "aliases".to_string(),
                            FieldSchema {
                                field_type: "array".to_string(),
                                item_type: Some("string".to_string()),
                                pattern: None,
                                values: None,
                                required: Some(true),
                                max_length: None,
                                description: "Search aliases".to_string(),
                            },
                        );
                        map
                    },
                    custom_field_allowed: CustomFieldConfig {
                        enabled: false,
                        max_one: false,
                        description: "".to_string(),
                        examples: None,
                    },
                    entries: vec![json!({
                        "id": "skill-docs",
                        "aliases": ["api-docs", "docs-generator"]
                    })],
                },
            ],
            validation: ValidationConfig {
                rules: ValidationRules {
                    alias_min_length: 2,
                    alias_max_length: 50,
                    description_min_length: 10,
                    description_max_length: 255,
                    custom_field_per_entry: 1,
                    required_base_fields: vec!["id".to_string(), "aliases".to_string()],
                },
                error_messages: HashMap::new(),
            },
        }
    }

    #[test]
    fn test_check_duplicate_aliases_no_duplicates() {
        let registry = make_test_registry();
        let validator = Validator::new(registry);

        // aliases that don't exist in registry
        let result = validator.check_duplicate_aliases(
            "projects",
            None,
            &["new-alias".to_string(), "another-one".to_string()],
        );
        assert!(result.is_empty());
    }

    #[test]
    fn test_check_duplicate_aliases_same_group() {
        let registry = make_test_registry();
        let validator = Validator::new(registry);

        // "alpha" already exists in projects
        let result = validator.check_duplicate_aliases(
            "projects",
            None,
            &["alpha".to_string()],
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "DUPLICATE_ALIAS");
    }

    #[test]
    fn test_check_duplicate_aliases_cross_group() {
        let registry = make_test_registry();
        let validator = Validator::new(registry);

        // "api-docs" exists in skills group
        let result = validator.check_duplicate_aliases(
            "projects",
            None,
            &["api-docs".to_string()],
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "DUPLICATE_ALIAS");
    }

    #[test]
    fn test_check_duplicate_aliases_case_insensitive() {
        let registry = make_test_registry();
        let validator = Validator::new(registry);

        // "ALPHA" should match "alpha" (case insensitive)
        let result = validator.check_duplicate_aliases(
            "projects",
            None,
            &["ALPHA".to_string()],
        );
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_check_duplicate_aliases_skip_self_on_edit() {
        let registry = make_test_registry();
        let validator = Validator::new(registry);

        // When editing proj-alpha, its own aliases should not be flagged
        let result = validator.check_duplicate_aliases(
            "projects",
            Some("proj-alpha"),
            &["alpha".to_string(), "project-alpha".to_string()],
        );
        assert!(result.is_empty());
    }

    #[test]
    fn test_check_duplicate_aliases_catch_others_on_edit() {
        let registry = make_test_registry();
        let validator = Validator::new(registry);

        // When editing proj-alpha, trying to use beta's alias should fail
        let result = validator.check_duplicate_aliases(
            "projects",
            Some("proj-alpha"),
            &["beta".to_string()],
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "DUPLICATE_ALIAS");
    }

    #[test]
    fn test_check_duplicate_aliases_multiple_duplicates() {
        let registry = make_test_registry();
        let validator = Validator::new(registry);

        // Both "alpha" and "beta" exist in projects
        let result = validator.check_duplicate_aliases(
            "projects",
            None,
            &["alpha".to_string(), "beta".to_string()],
        );
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_validate_entry_valid_project() {
        let registry = make_test_registry();
        let validator = Validator::new(registry);

        let entry = json!({
            "id": "proj-gamma",
            "aliases": ["gamma", "project-gamma"]
        });

        let result = validator.validate_entry("projects", &entry);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_entry_missing_required_field() {
        let registry = make_test_registry();
        let validator = Validator::new(registry);

        let entry = json!({
            "aliases": ["gamma"]
        });

        let result = validator.validate_entry("projects", &entry);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors[0].code, "MISSING_REQUIRED_FIELD");
    }

    #[test]
    fn test_validate_entry_invalid_alias_type() {
        let registry = make_test_registry();
        let validator = Validator::new(registry);

        let entry = json!({
            "id": "proj-gamma",
            "aliases": "not-an-array"
        });

        let result = validator.validate_entry("projects", &entry);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors[0].code, "INVALID_TYPE");
    }

    #[test]
    fn test_validate_entry_alias_too_short() {
        let registry = make_test_registry();
        let validator = Validator::new(registry);

        let entry = json!({
            "id": "proj-gamma",
            "aliases": ["a"] // min length is 2
        });

        let result = validator.validate_entry("projects", &entry);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors[0].code, "ALIAS_TOO_SHORT");
    }

    #[test]
    fn test_validate_entry_group_not_found() {
        let registry = make_test_registry();
        let validator = Validator::new(registry);

        let entry = json!({
            "id": "proj-gamma",
            "aliases": ["gamma"]
        });

        let result = validator.validate_entry("nonexistent", &entry);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors[0].code, "GROUP_NOT_FOUND");
    }
}
