use csv::ReaderBuilder;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::Path;

pub mod error;
pub mod scanner;
pub mod schema;
pub mod search;
pub mod validator;

// Re-export commonly used types
pub use error::{ValidationError, ValidatorError};
pub use schema::{KeywordGroup, KeywordRegistry, SearchResult};
pub use search::KeywordSearch;
pub use validator::Validator;

/// โหลด schema จากไฟล์ JSON หรือ YAML (พร้อมตรวจสอบความปลอดภัยเบื้องต้น)
pub fn load_registry<P: AsRef<Path>>(path: P) -> Result<KeywordRegistry, ValidatorError> {
    let path = path.as_ref();

    // ป้องกัน Path Traversal เบื้องต้น
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(ValidatorError::FileIo(
            "Access denied: Illegal path components".to_string(),
        ));
    }

    let content = fs::read_to_string(path).map_err(|_| {
        ValidatorError::FileIo(
            "Failed to read registry file: Access denied or not found".to_string(),
        )
    })?;

    let registry: KeywordRegistry = if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml") {
            serde_yaml::from_str(&content)
                .map_err(|e| ValidatorError::FileIo(format!("YAML parsing error: {}", e)))?
        } else {
            serde_json::from_str(&content)?
        }
    } else {
        serde_json::from_str(&content)?
    };

    Ok(registry)
}

/// save registry ลงไฟล์ JSON (พร้อมตรวจสอบความปลอดภัยเบื้องต้น)
pub fn save_registry<P: AsRef<Path>>(
    path: P,
    registry: &KeywordRegistry,
) -> Result<(), ValidatorError> {
    let path = path.as_ref();

    // ป้องกัน Path Traversal เบื้องต้น
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(ValidatorError::FileIo(
            "Access denied: Illegal path components".to_string(),
        ));
    }

    let json = serde_json::to_string_pretty(registry).map_err(ValidatorError::JsonError)?;

    fs::write(path, json).map_err(|_| {
        ValidatorError::FileIo("Failed to write registry file: Access denied".to_string())
    })?;

    Ok(())
}

/// นำเข้าข้อมูลพจนานุกรมจากไฟล์ CSV
/// คอลัมน์ที่ต้องการ: id, keyword(en), keyword(th), meaning, group, collection
pub fn import_dictionary_csv<P: AsRef<Path>>(
    path: P,
    registry: &mut KeywordRegistry,
) -> Result<usize, ValidatorError> {
    let file = fs::File::open(path)
        .map_err(|e| ValidatorError::FileIo(format!("Failed to open CSV file: {}", e)))?;

    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(file);

    let headers = rdr
        .headers()
        .map_err(|e| ValidatorError::FileIo(format!("CSV Header error: {}", e)))?
        .clone();

    let mut imported_count = 0;

    for result in rdr.records() {
        let record =
            result.map_err(|e| ValidatorError::FileIo(format!("CSV Record error: {}", e)))?;

        let mut id = String::new();
        let mut keyword_en = String::new();
        let mut keyword_th = String::new();
        let mut meaning = String::new();
        let mut group_id = String::new();
        let mut collection = String::new();

        for (i, header) in headers.iter().enumerate() {
            let val = record.get(i).unwrap_or("").trim().to_string();
            match header.to_lowercase().as_str() {
                "id" => id = val,
                "keyword(en)" | "keyword_en" | "en" => keyword_en = val,
                "keyword(th)" | "keyword_th" | "th" => keyword_th = val,
                "meaning" | "description" => meaning = val,
                "group" | "category" => group_id = val,
                "collection" | "tags" => collection = val,
                _ => {}
            }
        }

        if id.is_empty() && keyword_en.is_empty() {
            continue;
        }
        if group_id.is_empty() {
            group_id = "keywords".to_string();
        }

        // ค้นหาหรือสร้างกลุ่ม
        let group = registry.groups.iter_mut().find(|g| g.group_id == group_id);

        let group = match group {
            Some(g) => g,
            None => {
                // สร้างกลุ่มใหม่ถ้าไม่พบ
                registry.groups.push(KeywordGroup {
                    group_id: group_id.clone(),
                    group_name: group_id.clone(),
                    description: format!("Auto-generated group for {}", group_id),
                    base_fields_schema: std::collections::HashMap::new(),
                    custom_field_allowed: Default::default(),
                    entries: Vec::new(),
                });
                registry.groups.last_mut().unwrap()
            }
        };

        // เตรียมข้อมูล Entry
        let mut entry = Map::new();
        entry.insert("id".to_string(), json!(id));

        let mut aliases = Vec::new();
        if !keyword_en.is_empty() {
            aliases.push(json!(keyword_en));
        }
        if !keyword_th.is_empty() {
            aliases.push(json!(keyword_th));
        }
        entry.insert("aliases".to_string(), Value::Array(aliases));

        entry.insert("description".to_string(), json!(meaning));
        entry.insert("status".to_string(), json!("active"));
        entry.insert("type".to_string(), json!("dictionary"));

        if !collection.is_empty() {
            let tags: Vec<Value> = collection.split(',').map(|s| json!(s.trim())).collect();
            entry.insert("tags".to_string(), Value::Array(tags));
        }

        group.entries.push(Value::Object(entry));
        imported_count += 1;
    }

    Ok(imported_count)
}

/// แปลง registry เป็น Markdown สำหรับแสดงเอกสาร
pub fn generate_markdown(registry: &KeywordRegistry) -> String {
    let mut md = String::new();

    md.push_str(&format!("# Keyword Registry (v{})\n\n", registry.version));
    md.push_str(&format!("{}\n\n", registry.metadata.description));
    md.push_str(&format!(
        "**Last Updated:** {}\n",
        registry.metadata.last_updated
    ));
    md.push_str(&format!("**Owner:** {}\n\n", registry.metadata.owner));

    md.push_str("## Table of Contents\n\n");
    for group in &registry.groups {
        md.push_str(&format!("- [{}](#{})\n", group.group_name, group.group_id));
    }
    md.push_str("\n---\n\n");

    for group in &registry.groups {
        md.push_str(&format!("<a name=\"{}\"></a>\n", group.group_id));
        md.push_str(&format!("## {}\n\n", group.group_name));
        md.push_str(&format!("> {}\n\n", group.description));

        md.push_str("| ID | Aliases | Description | Type/Status |\n");
        md.push_str("| :--- | :--- | :--- | :--- |\n");

        for entry in &group.entries {
            let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("-");
            let desc = entry
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("-");

            let aliases = entry
                .get("aliases")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|a| a.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "-".to_string());

            let type_str = entry.get("type").and_then(|v| v.as_str()).unwrap_or("-");
            let status = entry.get("status").and_then(|v| v.as_str()).unwrap_or("-");

            md.push_str(&format!(
                "| `{}` | {} | {} | {} / {} |\n",
                id, aliases, desc, type_str, status
            ));
        }
        md.push_str("\n\n");
    }

    md.push_str("---\n*Generated by bl1nk-keyword-validator*\n");
    md
}
