pub mod error;
pub mod schema;
pub mod search;
pub mod validator;

use std::fs;
use std::path::Path;

// Re-export commonly used types
pub use schema::{KeywordRegistry, KeywordGroup, SearchResult};
pub use validator::Validator;
pub use search::KeywordSearch;
pub use error::{ValidatorError, ValidationError};

/// โหลด schema จากไฟล์ JSON (พร้อมตรวจสอบความปลอดภัยเบื้องต้น)
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
        ValidatorError::FileIo("Failed to read registry file: Access denied or not found".to_string())
    })?;

    let registry: KeywordRegistry = serde_json::from_str(&content)?;
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
