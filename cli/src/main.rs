use bl1nk_keyword_core::{load_registry, save_registry, KeywordSearch, Validator};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "keyword-registry",
    version = "1.1.0",
    about = "Validation and search tool for bl1nk keyword registry"
)]
struct Cli {
    #[arg(
        global = true,
        short,
        long,
        default_value = "./keyword-registry.json",
        help = "Path to keyword registry JSON file"
    )]
    schema: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate entire schema or single entry
    Validate {
        /// Entry ID to validate (optional, validates entire schema if not provided)
        #[arg(value_name = "ID")]
        entry_id: Option<String>,

        /// Group ID (required if validating single entry)
        #[arg(short, long)]
        group: Option<String>,
    },

    /// Search keyword by query
    Search {
        /// Search query (supports Thai and English)
        #[arg(value_name = "QUERY")]
        query: String,

        /// Filter by Group ID
        #[arg(short, long)]
        group: Option<String>,

        /// Return raw JSON output
        #[arg(short, long)]
        json: bool,
    },

    /// Add new entry to registry
    Add {
        /// Group ID (projects, skills, keywords)
        #[arg(value_name = "GROUP")]
        group: String,

        /// JSON entry data as string or @file
        #[arg(value_name = "JSON")]
        entry: String,
    },

    /// Edit existing entry
    Edit {
        /// Entry ID to edit
        #[arg(value_name = "ID")]
        id: String,

        /// Group ID
        #[arg(short, long)]
        group: String,

        /// Field name to edit
        #[arg(short, long)]
        field: String,

        /// New value
        #[arg(short, long)]
        value: String,
    },

    /// Show entry details
    Show {
        /// Entry ID
        #[arg(value_name = "ID")]
        id: String,

        /// Return raw JSON output
        #[arg(short, long)]
        json: bool,
    },
/// List all entries in a group
List {
    /// Group ID
    #[arg(value_name = "GROUP")]
    group: String,

    /// Return raw JSON output
    #[arg(short, long)]
    json: bool,
},

/// Export JSON Schema for the registry
SchemaExport,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // schema-export ไม่ต้องโหลด registry ก่อน
    if let Commands::SchemaExport = cli.command {
        let schema = schemars::schema_for!(bl1nk_keyword_core::schema::KeywordRegistry);
        println!("{}", serde_json::to_string_pretty(&schema)?);
        return Ok(());
    }

    // โหลด registry
    let registry =
        load_registry(&cli.schema).map_err(|e| format!("Failed to load schema: {}", e))?;

    match cli.command {
        Commands::Validate { entry_id, group } => {
            cmd_validate(&registry, entry_id, group)?;
        }

        Commands::Search { query, group, json } => {
            cmd_search(&registry, &query, group, json)?;
        }

        Commands::Add { group, entry } => {
            cmd_add(&registry, &cli.schema, &group, &entry)?;
        }

        Commands::Edit {
            id,
            group,
            field,
            value,
        } => {
            cmd_edit(&registry, &cli.schema, &id, &group, &field, &value)?;
        }

        Commands::Show { id, json } => {
            cmd_show(&registry, &id, json)?;
        }

        Commands::List { group, json } => {
            cmd_list(&registry, &group, json)?;
        }

        Commands::SchemaExport => unreachable!(),
    }

    Ok(())
}

fn cmd_validate(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    entry_id: Option<String>,
    group: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::new(registry.clone());

    if let Some(id) = entry_id {
        let group_id = group.ok_or("Group ID required when validating single entry")?;

        let target_group = registry
            .groups
            .iter()
            .find(|g| g.group_id == group_id)
            .ok_or("Group not found")?;

        let entry = target_group
            .entries
            .iter()
            .find(|e| e.get("id").and_then(|v| v.as_str()) == Some(&id))
            .ok_or("Entry not found")?;

        match validator.validate_entry(&group_id, entry) {
            Ok(_) => {
                println!("{}", json!({ "valid": true, "message": "Entry is valid" }));
            }
            Err(errors) => {
                let response = json!({
                    "valid": false,
                    "errors": errors
                });
                println!("{}", response);
                std::process::exit(1);
            }
        }
    } else {
        // validate ทั้งหมด
        let mut all_valid = true;

        for group in &registry.groups {
            for entry in &group.entries {
                if let Err(errors) = validator.validate_entry(&group.group_id, entry) {
                    all_valid = false;
                    let response = json!({
                        "valid": false,
                        "entry": entry.get("id"),
                        "group": group.group_id,
                        "errors": errors
                    });
                    println!("{}", response);
                }
            }
        }

        if all_valid {
            println!(
                "{}",
                json!({ "valid": true, "message": "All entries are valid" })
            );
        } else {
            std::process::exit(1);
        }
    }

    Ok(())
}

fn cmd_search(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    query: &str,
    group_id: Option<String>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let search = KeywordSearch::new(registry.clone());
    let results = search.search(query, group_id.as_deref());

    if json_output {
        let response = json!({
            "query": query,
            "results": results,
            "count": results.len()
        });
        println!("{}", response);
    } else {
        if results.is_empty() {
            println!("No results found for: {}", query);
        } else {
            println!("Found {} result(s):\n", results.len());
            for result in results {
                println!("ID: {}", result.id);
                println!("Group: {}", result.group_id);
                println!("Match Type: {} (Score: {})", result.match_type, result.score);
                println!("Aliases: {}", result.aliases.join(", "));
                println!("Description: {}", result.description);
                println!("---");
            }
        }
    }

    Ok(())
}

fn cmd_add(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    schema_path: &std::path::PathBuf,
    group: &str,
    entry_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reg = registry.clone();

    // parse entry JSON
    let entry: Value = if let Some(file_path) = entry_str.strip_prefix('@') {
        let content = std::fs::read_to_string(file_path)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::from_str(entry_str)?
    };

    let validator = Validator::new(reg.clone());

    // validate entry
    if let Err(errors) = validator.validate_entry(group, &entry) {
        let response = json!({
            "valid": false,
            "errors": errors
        });
        println!("{}", response);
        std::process::exit(1);
    }

    // check duplicate aliases
    if let Some(aliases_arr) = entry.get("aliases").and_then(|v| v.as_array()) {
        let aliases: Vec<String> = aliases_arr
            .iter()
            .filter_map(|a| a.as_str().map(String::from))
            .collect();

        if !aliases.is_empty() {
            let alias_errors = validator.check_duplicate_aliases(group, None, &aliases);
            if !alias_errors.is_empty() {
                let response = json!({
                    "valid": false,
                    "errors": alias_errors
                });
                println!("{}", response);
                std::process::exit(1);
            }
        }
    }

    // add เข้า registry
    if let Some(target_group) = reg.groups.iter_mut().find(|g| g.group_id == group) {
        let entry_id = entry
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        target_group.entries.push(entry.clone());

        save_registry(schema_path, &reg)?;

        let response = json!({
            "success": true,
            "groupId": group,
            "id": entry_id,
            "message": "Entry added successfully"
        });
        println!("{}", response);
    } else {
        let response = json!({
            "valid": false,
            "error": "Group not found"
        });
        println!("{}", response);
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_edit(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    schema_path: &std::path::PathBuf,
    id: &str,
    group: &str,
    field: &str,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reg = registry.clone();

    // หา entry และแก้ไขค่า
    let entry_id = {
        let target_group = reg
            .groups
            .iter_mut()
            .find(|g| g.group_id == group)
            .ok_or("Group not found")?;

        let entry = target_group
            .entries
            .iter_mut()
            .find(|e| e.get("id").and_then(|v| v.as_str()) == Some(id))
            .ok_or("Entry not found")?;

        // parse value ตามประเภท
        let parsed_value: Value = if value == "true" {
            Value::Bool(true)
        } else if value == "false" {
            Value::Bool(false)
        } else if let Ok(num) = value.parse::<i64>() {
            Value::Number(num.into())
        } else if value.starts_with('[') || value.starts_with('{') {
            serde_json::from_str(value)?
        } else {
            Value::String(value.to_string())
        };

        entry[field] = parsed_value;
        entry
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(id)
            .to_string()
    };

    // validate หลังแก้ไข (mutable borrow จบแล้ว)
    let target_group = reg
        .groups
        .iter()
        .find(|g| g.group_id == group)
        .ok_or("Group not found")?;

    let entry = target_group
        .entries
        .iter()
        .find(|e| e.get("id").and_then(|v| v.as_str()) == Some(&entry_id))
        .ok_or("Entry not found")?;

    let validator = Validator::new(reg.clone());
    if let Err(errors) = validator.validate_entry(group, entry) {
        let response = json!({
            "valid": false,
            "errors": errors
        });
        println!("{}", response);
        std::process::exit(1);
    }

    // check duplicate aliases (เฉพาะตอนแก้ field aliases)
    if field == "aliases" {
        if let Some(aliases_arr) = entry.get("aliases").and_then(|v| v.as_array()) {
            let aliases: Vec<String> = aliases_arr
                .iter()
                .filter_map(|a| a.as_str().map(String::from))
                .collect();

            if !aliases.is_empty() {
                let alias_errors =
                    validator.check_duplicate_aliases(group, Some(&entry_id), &aliases);
                if !alias_errors.is_empty() {
                    let response = json!({
                        "valid": false,
                        "errors": alias_errors
                    });
                    println!("{}", response);
                    std::process::exit(1);
                }
            }
        }
    }

    save_registry(schema_path, &reg)?;

    let response = json!({
        "success": true,
        "groupId": group,
        "id": id,
        "field": field,
        "message": "Entry updated successfully"
    });
    println!("{}", response);

    Ok(())
}

fn cmd_show(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    id: &str,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut found = false;

    for group in &registry.groups {
        for entry in &group.entries {
            if entry.get("id").and_then(|v| v.as_str()) == Some(id) {
                if json_output {
                    println!("{}", serde_json::to_string_pretty(entry)?);
                } else {
                    println!(
                        "ID: {}",
                        entry.get("id").and_then(|v| v.as_str()).unwrap_or("")
                    );
                    println!("Group: {}", group.group_id);
                    if let Some(aliases) = entry.get("aliases").and_then(|v| v.as_array()) {
                        println!(
                            "Aliases: {}",
                            aliases
                                .iter()
                                .filter_map(|a| a.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                    if let Some(desc) = entry.get("description").and_then(|v| v.as_str()) {
                        println!("Description: {}", desc);
                    }
                    println!("\nFull entry:");
                    println!("{}", serde_json::to_string_pretty(entry)?);
                }
                found = true;
                break;
            }
        }
        if found {
            break;
        }
    }

    if !found {
        eprintln!("Entry not found: {}", id);
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_list(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    group: &str,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let target_group = registry
        .groups
        .iter()
        .find(|g| g.group_id == group)
        .ok_or("Group not found")?;

    if json_output {
        let ids: Vec<&str> = target_group
            .entries
            .iter()
            .filter_map(|e| e.get("id").and_then(|v| v.as_str()))
            .collect();

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "group": group,
                "count": ids.len(),
                "entries": ids
            }))?
        );
    } else {
        println!("Group: {} ({})", group, target_group.group_name);
        println!("Entries: {}\n", target_group.entries.len());

        for entry in &target_group.entries {
            if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
                if let Some(desc) = entry.get("description").and_then(|v| v.as_str()) {
                    println!("  - {}: {}", id, desc);
                }
            }
        }
    }

    Ok(())
}
