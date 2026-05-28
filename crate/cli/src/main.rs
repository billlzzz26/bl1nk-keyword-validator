use bl1nk_keyword_core::{count_usage_in_dir, load_registry, save_registry, get_top_keywords, KeywordSearch, Validator};
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

    /// Count keyword usage in files or directory
    Usage {
        /// Path to file or directory to scan
        #[arg(value_name = "PATH")]
        path: PathBuf,

        /// Filter by file extension (e.g. rs,md,json)
        #[arg(short, long)]
        extensions: Option<String>,

        /// Show top N keywords (default: 10)
        #[arg(short, long, default_value = "10")]
        top: usize,

        /// Return raw JSON output
        #[arg(short, long)]
        json: bool,
    },

    /// Generate Markdown documentation from the registry
    DocsGen {
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // schema-export ไม่ต้องโหลด registry ก่อน
    if let Commands::SchemaExport = cli.command {
        let schema = schemars::schema_for!(bl1nk_keyword_core::schema::KeywordRegistry);
        println!("{}", serde_json::to_string_pretty(&schema)?);
        return Ok(());
    }

    // โหลด registry
    let registry = load_registry(&cli.schema).map_err(|e| match e {
        bl1nk_keyword_core::ValidatorError::FileIo(msg) => msg,
        bl1nk_keyword_core::ValidatorError::JsonError(err) => format!("Data format error: {}", err),
        _ => format!("Registry error: {}", e),
    })?;

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

        Commands::Usage { path, extensions, top, json } => {
            cmd_usage(&registry, &path, extensions.as_deref(), top, json)?;
        }

        Commands::DocsGen { output } => {
            cmd_docs_gen(&registry, output)?;
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
        // validate single entry
        let g_id = group.ok_or("Group ID is required for single entry validation")?;
        let group_data = registry
            .groups
            .iter()
            .find(|g| g.group_id == g_id)
            .ok_or(format!("Group '{}' not found", g_id))?;

        let entry = group_data
            .entries
            .iter()
            .find(|e| e.get("id").and_then(|v| v.as_str()) == Some(&id))
            .ok_or(format!("Entry '{}' not found in group '{}'", id, g_id))?;

        match validator.validate_entry(&g_id, entry) {
            Ok(_) => {
                println!(
                    "{}",
                    json!({ "valid": true, "message": format!("Entry '{}' is valid", id) })
                );
            }
            Err(errors) => {
                println!("{}", json!({ "valid": false, "errors": errors }));
                std::process::exit(1);
            }
        }
    } else {
        // validate entire registry
        match validator.validate_registry() {
            Ok(_) => {
                println!(
                    "{}",
                    json!({ "valid": true, "message": "All entries are valid" })
                );
            }
            Err(errors) => {
                println!("{}", json!({ "valid": false, "errors": errors }));
                std::process::exit(1);
            }
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
                println!(
                    "Match Type: {} (Score: {})",
                    result.match_type, result.score
                );
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
    let mut registry = registry.clone();
    let new_entry: Value = if let Some(path) = entry_str.strip_prefix('@') {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::from_str(entry_str)?
    };

    let validator = Validator::new(registry.clone());

    // 1. ตรวจสอบความถูกต้องของ entry
    validator.validate_entry(group, &new_entry).map_err(|e| {
        format!(
            "Validation failed: {}",
            serde_json::to_string_pretty(&e).unwrap()
        )
    })?;

    // 2. ตรวจสอบ duplicate aliases
    if let Some(aliases) = new_entry.get("aliases").and_then(|v| v.as_array()) {
        let alias_strings: Vec<String> = aliases
            .iter()
            .filter_map(|a| a.as_str().map(String::from))
            .collect();

        let dup_errors = validator.check_duplicate_aliases(group, None, &alias_strings);
        if !dup_errors.is_empty() {
            return Err(format!(
                "Duplicate aliases found: {}",
                serde_json::to_string_pretty(&dup_errors).unwrap()
            )
            .into());
        }
    }

    // 3. เพิ่มเข้า registry
    let group_data = registry
        .groups
        .iter_mut()
        .find(|g| g.group_id == group)
        .ok_or(format!("Group '{}' not found", group))?;

    group_data.entries.push(new_entry);

    // 4. บันทึกไฟล์
    save_registry(schema_path, &registry)?;
    println!(
        "{}",
        json!({ "valid": true, "message": "Entry added successfully" })
    );

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
    let mut registry = registry.clone();
    let validator = Validator::new(registry.clone());

    let group_data = registry
        .groups
        .iter_mut()
        .find(|g| g.group_id == group)
        .ok_or(format!("Group '{}' not found", group))?;

    let entry = group_data
        .entries
        .iter_mut()
        .find(|e| e.get("id").and_then(|v| v.as_str()) == Some(id))
        .ok_or(format!("Entry '{}' not found in group '{}'", id, group))?;

    // อัปเดตค่า (จัดการประเภทข้อมูลเบื้องต้น)
    let new_val: Value = if value.starts_with('[') || value.starts_with('{') {
        serde_json::from_str(value)?
    } else {
        Value::String(value.to_string())
    };

    entry[field] = new_val;

    // ตรวจสอบความถูกต้องหลังแก้ไข
    validator.validate_entry(group, entry).map_err(|e| {
        format!(
            "Validation failed after edit: {}",
            serde_json::to_string_pretty(&e).unwrap()
        )
    })?;

    // ตรวจสอบ duplicate aliases (ถ้าแก้ไข aliases)
    if field == "aliases" {
        if let Some(aliases) = entry.get("aliases").and_then(|v| v.as_array()) {
            let alias_strings: Vec<String> = aliases
                .iter()
                .filter_map(|a| a.as_str().map(String::from))
                .collect();

            let dup_errors = validator.check_duplicate_aliases(group, Some(id), &alias_strings);
            if !dup_errors.is_empty() {
                return Err(format!(
                    "Duplicate aliases found after edit: {}",
                    serde_json::to_string_pretty(&dup_errors).unwrap()
                )
                .into());
            }
        }
    }

    save_registry(schema_path, &registry)?;
    println!(
        "{}",
        json!({ "valid": true, "message": "Entry updated successfully" })
    );

    Ok(())
}

fn cmd_show(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    id: &str,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    for group in &registry.groups {
        if let Some(entry) = group
            .entries
            .iter()
            .find(|e| e.get("id").and_then(|v| v.as_str()) == Some(id))
        {
            if json_output {
                println!("{}", serde_json::to_string_pretty(entry)?);
            } else {
                println!("Entry Details:");
                println!("ID: {}", id);
                println!("Group: {}", group.group_id);
                println!("{:#}", entry);
            }
            return Ok(());
        }
    }

    Err(format!("Entry '{}' not found in any group", id).into())
}

fn cmd_list(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    group_id: &str,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let group = registry
        .groups
        .iter()
        .find(|g| g.group_id == group_id)
        .ok_or(format!("Group '{}' not found", group_id))?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&group.entries)?);
    } else {
        println!("Entries in group '{}':\n", group_id);
        for entry in &group.entries {
            if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
                if let Some(desc) = entry.get("description").and_then(|v| v.as_str()) {
                    println!("- {}: {}", id, desc);
                }
            }
        }
    }

    Ok(())
}

fn cmd_docs_gen(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let markdown = bl1nk_keyword_core::generate_markdown(registry);

    match output {
        Some(path) => {
            std::fs::write(&path, markdown)?;
            println!("Documentation generated at: {}", path.display());
        }
        None => {
            println!("{}", markdown);
        }
    }

    Ok(())
}

fn cmd_usage(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    path: &std::path::Path,
    extensions: Option<&str>,
    top: usize,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let exts: Option<Vec<&str>> = extensions.map(|s| {
        s.split(',')
            .filter(|s| !s.is_empty())
            .collect()
    });
    let ext_slice: Option<&[&str]> = exts.as_deref();

    let stats = count_usage_in_dir(path, registry, ext_slice)?;

    if json_output {
        println!("{}", json!({
            "path": path.to_string_lossy(),
            "total_keywords_found": stats.len(),
            "top_keywords": get_top_keywords(&stats, top),
            "stats": stats
        }));
    } else {
        let tops = get_top_keywords(&stats, top);
        println!("Keyword usage in {}:\n", path.to_string_lossy());
        if tops.is_empty() {
            println!("No keywords found.");
        } else {
            println!("Top {} keywords:\n", top);
            for (id, count) in tops {
                println!("  {}: {} occurrences", id, count);
            }
        }
    }

    Ok(())
}
