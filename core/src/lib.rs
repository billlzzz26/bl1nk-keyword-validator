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

/// โหลด schema จากไฟล์ JSON
pub fn load_registry<P: AsRef<Path>>(path: P) -> Result<KeywordRegistry, ValidatorError> {
    let content = fs::read_to_string(path)
        .map_err(|e| ValidatorError::FileIo(format!("Failed to read file: {}", e)))?;

    let registry: KeywordRegistry = serde_json::from_str(&content)?;
    Ok(registry)
}

/// save registry ลงไฟล์ JSON
pub fn save_registry<P: AsRef<Path>>(
    path: P,
    registry: &KeywordRegistry,
) -> Result<(), ValidatorError> {
    let json = serde_json::to_string_pretty(registry).map_err(ValidatorError::JsonError)?;

    fs::write(path, json)
        .map_err(|e| ValidatorError::FileIo(format!("Failed to write file: {}", e)))?;

    Ok(())
}
