use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============= Schema Types =============

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct KeywordRegistry {
    pub version: String,
    pub metadata: Metadata,
    #[serde(default)]
    pub groups: Vec<KeywordGroup>,
    #[serde(default)]
    pub validation: ValidationConfig,
    /// v0.3.0+ Thai-English language mapping (optional)
    #[serde(rename = "languageMapping", default, skip_serializing_if = "Option::is_none")]
    pub language_mapping: Option<LanguageMapping>,
    /// v0.3.0+ Detection system configuration (optional)  
    #[serde(rename = "detectionSystem", default, skip_serializing_if = "Option::is_none")]
    pub detection_system: Option<DetectionSystem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Metadata {
    #[serde(rename = "lastUpdated")]
    #[serde(default)]
    pub last_updated: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct KeywordGroup {
    #[serde(rename = "groupId")]
    pub group_id: String,
    #[serde(rename = "groupName")]
    pub group_name: String,
    pub description: String,
    #[serde(rename = "baseFieldsSchema", default)]
    pub base_fields_schema: HashMap<String, FieldSchema>,
    #[serde(rename = "customFieldAllowed")]
    pub custom_field_allowed: CustomFieldConfig,
    pub entries: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct FieldSchema {
    #[serde(rename = "type")]
    #[serde(default)]
    pub field_type: String,
    #[serde(rename = "itemType", skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(rename = "maxLength", skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct CustomFieldConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(rename = "maxOne")]
    #[serde(default)]
    pub max_one: bool,
    #[serde(default)]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct ValidationConfig {
    pub rules: ValidationRules,
    #[serde(rename = "errorMessages")]
    pub error_messages: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct ValidationRules {
    #[serde(rename = "aliasMinLength")]
    #[serde(default)]
    pub alias_min_length: usize,
    #[serde(rename = "aliasMaxLength")]
    #[serde(default)]
    pub alias_max_length: usize,
    #[serde(rename = "descriptionMinLength")]
    #[serde(default)]
    pub description_min_length: usize,
    #[serde(rename = "descriptionMaxLength")]
    #[serde(default)]
    pub description_max_length: usize,
    #[serde(rename = "customFieldPerEntry")]
    #[serde(default)]
    pub custom_field_per_entry: usize,
    #[serde(rename = "requiredBaseFields")]
    #[serde(default)]
    pub required_base_fields: Vec<String>,
}

// ============= Search Types =============

/// Options for controlling search behavior.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Filter by group ID (None = search all groups)
    pub group_id: Option<String>,
    /// Minimum score threshold (0.0-1.0). Results below this are filtered out.
    pub min_score: f64,
    /// Maximum number of results to return.
    pub max_results: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            group_id: None,
            min_score: 0.05,
            max_results: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    pub id: String,
    #[serde(rename = "groupId")]
    pub group_id: String,
    pub aliases: Vec<String>,
    pub description: String,
    #[serde(rename = "matchType")]
    pub match_type: String, // "exact", "keyboard_fix", "edit_distance", "partial", "fuzzy"
    /// Normalized score (0.0-1.0). Higher = better match.
    pub score: f64,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub count: usize,
}

// ============= v0.3.0+ Extension Types =============

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct LanguageMapping {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub mapping_strategies: Vec<MappingStrategy>,
    #[serde(default)]
    pub thai_english_dictionary: Vec<LanguageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct MappingStrategy {
    #[serde(default)]
    pub strategy: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub example: LanguageExample,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct LanguageExample {
    #[serde(default)]
    pub thai: String,
    #[serde(default)]
    pub english: String,
    #[serde(default)]
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct LanguageEntry {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub thai: String,
    #[serde(default)]
    pub english: String,
    #[serde(default)]
    pub alternates: LanguageAlternates,
    #[serde(default)]
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct LanguageAlternates {
    #[serde(default)]
    pub thai: Vec<String>,
    #[serde(default)]
    pub english: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct DetectionSystem {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_number_detection: Option<LineNumberDetection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_search: Option<TextSearch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fts5_search: Option<Fts5Search>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct LineNumberDetection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct TextSearch {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Fts5Search {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub indexed_fields: Vec<String>,
}
