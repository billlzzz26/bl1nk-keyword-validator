use crate::error::ValidationError;
use crate::schema::{FieldSchema, KeywordRegistry};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashSet;

/// หลักการ validate entry ตามโครงสร้าง schema
pub struct Validator {
    registry: KeywordRegistry,
}

impl Validator {
    pub fn new(registry: KeywordRegistry) -> Self {
        Self { registry }
    }

    /// validate ข้อมูล entry ทั้งหมด
    pub fn validate_entry(
        &self,
        group_id: &str,
        entry: &Value,
    ) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // หา group จาก registry
        let group = match self.registry.groups.iter().find(|g| g.group_id == group_id) {
            Some(g) => g,
            None => {
                errors.push(ValidationError {
                    code: "GROUP_NOT_FOUND".to_string(),
                    field: Some("group_id".to_string()),
                    message: format!("Group '{}' not found", group_id),
                });
                return Err(errors);
            }
        };

        // validate base fields
        for (field_name, field_schema) in &group.base_fields_schema {
            if let Some(is_required) = field_schema.required {
                if is_required && entry.get(field_name).is_none() {
                    errors.push(ValidationError {
                        code: "MISSING_REQUIRED_FIELD".to_string(),
                        field: Some(field_name.clone()),
                        message: format!("Required field missing: {}", field_name),
                    });
                    continue;
                }
            }

            if let Some(value) = entry.get(field_name) {
                if let Err(e) = self.validate_field(field_name, value, field_schema) {
                    errors.extend(e);
                }
            }
        }

        // validate custom field (max 1)
        if group.custom_field_allowed.enabled {
            let mut custom_fields = 0;
            for key in entry.as_object().unwrap().keys() {
                if !group.base_fields_schema.contains_key(key) {
                    custom_fields += 1;
                }
            }

            let max_allowed = if group.custom_field_allowed.max_one { 1 } else { 100 }; // fallback to a large number if not max_one
            if custom_fields > max_allowed {
                errors.push(ValidationError {
                    code: "TOO_MANY_CUSTOM_FIELDS".to_string(),
                    field: None,
                    message: format!(
                        "Maximum {} custom field allowed",
                        max_allowed
                    ),
                });
            }
        }

        // validate id ไม่ซ้ำ (ยกเว้นตัวเองตอน update)
        if let Some(id_value) = entry.get("id") {
            if let Some(id_str) = id_value.as_str() {
                for existing in &group.entries {
                    if let Some(existing_id) = existing.get("id").and_then(|v| v.as_str()) {
                        // ข้าม entry ตัวเอง (กรณี update)
                        if existing_id == id_str && existing != entry {
                            errors.push(ValidationError {
                                code: "DUPLICATE_ID".to_string(),
                                field: Some("id".to_string()),
                                message: format!("ID already exists: {}", id_str),
                            });
                            break;
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// validate single field ตามโครงสร้าง
    fn validate_field(
        &self,
        field_name: &str,
        value: &Value,
        schema: &FieldSchema,
    ) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        match schema.field_type.as_str() {
            "string" => {
                if !value.is_string() {
                    errors.push(ValidationError {
                        code: "INVALID_TYPE".to_string(),
                        field: Some(field_name.to_string()),
                        message: format!(
                            "Field type mismatch. Expected string, got {}",
                            value.type_str()
                        ),
                    });
                    return Err(errors);
                }

                let s = value.as_str().unwrap();

                // ตรวจ pattern (regex)
                if let Some(pattern) = &schema.pattern {
                    if let Ok(re) = Regex::new(pattern) {
                        if !re.is_match(s) {
                            errors.push(ValidationError {
                                code: "INVALID_PATTERN".to_string(),
                                field: Some(field_name.to_string()),
                                message: format!(
                                    "Value must match pattern: {}",
                                    pattern
                                ),
                            });
                        }
                    }
                }

                // ตรวจ maxLength
                if let Some(max_len) = schema.max_length {
                    if s.len() > max_len {
                        errors.push(ValidationError {
                            code: "DESCRIPTION_TOO_LONG".to_string(),
                            field: Some(field_name.to_string()),
                            message: format!("Description exceeds {} characters", max_len),
                        });
                    }
                }
            }

            "array" => {
                if !value.is_array() {
                    errors.push(ValidationError {
                        code: "INVALID_TYPE".to_string(),
                        field: Some(field_name.to_string()),
                        message: format!(
                            "Field type mismatch. Expected array, got {}",
                            value.type_str()
                        ),
                    });
                    return Err(errors);
                }

                // ตรวจ array items
                if let Some(item_type) = &schema.item_type {
                    for (idx, item) in value.as_array().unwrap().iter().enumerate() {
                        match item_type.as_str() {
                            "string" => {
                                if !item.is_string() {
                                    errors.push(ValidationError {
                                        code: "INVALID_ARRAY_ITEM".to_string(),
                                        field: Some(format!("{}[{}]", field_name, idx)),
                                        message: "Array item must be string".to_string(),
                                    });
                                } else {
                                    // validate each alias length
                                    let alias_str = item.as_str().unwrap();
                                    if alias_str.len() < self.registry.validation.rules.alias_min_length {
                                        errors.push(ValidationError {
                                            code: "ALIAS_TOO_SHORT".to_string(),
                                            field: Some(format!("{}[{}]", field_name, idx)),
                                            message: format!(
                                                "Alias must be at least {} characters",
                                                self.registry.validation.rules.alias_min_length
                                            ),
                                        });
                                    }
                                    if alias_str.len() > self.registry.validation.rules.alias_max_length {
                                        errors.push(ValidationError {
                                            code: "ALIAS_TOO_LONG".to_string(),
                                            field: Some(format!("{}[{}]", field_name, idx)),
                                            message: format!(
                                                "Alias must not exceed {} characters",
                                                self.registry.validation.rules.alias_max_length
                                            ),
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            "enum" => {
                if let Some(allowed_values) = &schema.values {
                    let value_str = value.as_str().unwrap_or("");
                    if !allowed_values.contains(&value_str.to_string()) {
                        errors.push(ValidationError {
                            code: "INVALID_ENUM".to_string(),
                            field: Some(field_name.to_string()),
                            message: format!(
                                "Value must be one of: {}",
                                allowed_values.join(", ")
                            ),
                        });
                    }
                }
            }

            _ => {
                errors.push(ValidationError {
                    code: "UNKNOWN_TYPE".to_string(),
                    field: Some(field_name.to_string()),
                    message: format!("Unknown field type: {}", schema.field_type),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// check duplicate aliases ในทั้ง registry
    pub fn check_duplicate_aliases(&self, _group_id: &str, aliases: &[String]) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut seen_aliases = HashSet::new();

        for group in &self.registry.groups {
            for entry in &group.entries {
                if let Some(entry_aliases) = entry.get("aliases").and_then(|v| v.as_array()) {
                    for alias in entry_aliases {
                        if let Some(alias_str) = alias.as_str() {
                            seen_aliases.insert(alias_str.to_string());
                        }
                    }
                }
            }
        }

        for alias in aliases {
            if seen_aliases.contains(alias) {
                errors.push(ValidationError {
                    code: "DUPLICATE_ALIAS".to_string(),
                    field: Some("aliases".to_string()),
                    message: format!("This alias is already used: {}", alias),
                });
            }
        }

        errors
    }
}

// trait สำหรับ helper methods
trait ValueTypeStr {
    fn type_str(&self) -> &'static str;
}

impl ValueTypeStr for Value {
    fn type_str(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}
