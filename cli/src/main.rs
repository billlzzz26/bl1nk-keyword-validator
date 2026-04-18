//! เครื่องมือจัดการ Registry คำสำคัญของ bl1nk (bl1nk Keyword Registry Manager)
//!
//! ให้บริการอินเทอร์เฟซบรรทัดคำสั่ง (CLI) สำหรับการตรวจสอบ (Validation), ค้นหา (Search),
//! และจัดการไฟล์ JSON/YAML ที่เก็บข้อมูลคำสำคัญ (Keywords), โครงการ (Projects), และความสามารถ (Skills)

use anyhow::{Context, Result};
use bl1nk_keyword_core::{load_registry, save_registry, KeywordSearch, Validator};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::path::PathBuf;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(
    name = "keyword-registry",
    version = "1.1.0",
    about = "เครื่องมือตรวจสอบและค้นหาสำหรับ bl1nk keyword registry"
)]
struct Cli {
    /// พาธไปยังไฟล์ JSON ของ keyword registry
    #[arg(
        global = true,
        short,
        long,
        default_value = "./keyword-registry.json",
        help = "พาธไปยังไฟล์ JSON ของ keyword registry"
    )]
    schema: PathBuf,

    /// ไฟล์การกำหนดค่าเพิ่มเติมเพื่อเขียนทับหรือเพิ่มกฎการตรวจสอบ (YAML หรือ JSON)
    #[arg(
        global = true,
        short = 'c',
        long,
        help = "ไฟล์การกำหนดค่าเพิ่มเติมเพื่อเขียนทับหรือเพิ่มกฎการตรวจสอบ (YAML หรือ JSON)"
    )]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// ตรวจสอบความถูกต้องของสคีมาทั้งหมดหรือรายการเดียว
    Validate {
        /// ID ของรายการที่ต้องการตรวจสอบ (ถ้าไม่ระบุ จะตรวจสอบทั้งสคีมา)
        #[arg(value_name = "ID")]
        entry_id: Option<String>,

        /// ID ของกลุ่ม (จำเป็นต้องระบุหากตรวจสอบรายการเดียว)
        #[arg(short, long)]
        group: Option<String>,
    },

    /// ค้นหาคำสำคัญด้วยข้อความค้นหา
    Search {
        /// ข้อความค้นหา (รองรับภาษาไทยและภาษาอังกฤษ)
        #[arg(value_name = "QUERY")]
        query: String,

        /// กรองตาม ID ของกลุ่ม
        #[arg(short, long)]
        group: Option<String>,

        /// แสดงผลลัพธ์เป็น JSON ดิบ
        #[arg(short, long)]
        json: bool,
    },

    /// เพิ่มรายการใหม่เข้าไปใน registry
    Add {
        /// ID ของกลุ่ม (projects, skills, keywords)
        #[arg(value_name = "GROUP")]
        group: String,

        /// ข้อมูลรายการในรูปแบบ JSON string หรือใช้ @พาธไฟล์
        #[arg(value_name = "JSON")]
        entry: String,
    },

    /// แก้ไขรายการที่มีอยู่เดิม
    Edit {
        /// ID ของรายการที่ต้องการแก้ไข
        #[arg(value_name = "ID")]
        id: String,

        /// ID ของกลุ่ม
        #[arg(short, long)]
        group: String,

        /// ชื่อฟิลด์ที่ต้องการแก้ไข
        #[arg(short, long)]
        field: String,

        /// ค่าใหม่ที่ต้องการกำหนด
        #[arg(short, long)]
        value: String,
    },

    /// แสดงรายละเอียดของรายการ
    Show {
        /// ID ของรายการ
        #[arg(value_name = "ID")]
        id: String,

        /// แสดงผลลัพธ์เป็น JSON ดิบ
        #[arg(short, long)]
        json: bool,
    },

    /// รายการข้อมูลทั้งหมดในกลุ่ม
    List {
        /// ID ของกลุ่ม
        #[arg(value_name = "GROUP")]
        group: String,

        /// แสดงผลลัพธ์เป็น JSON ดิบ
        #[arg(short, long)]
        json: bool,
    },

    /// ส่งออก JSON Schema สำหรับ registry
    SchemaExport,

    /// สร้างเอกสาร Markdown จากข้อมูลใน registry
    DocsGen {
        /// พาธไฟล์สำหรับบันทึกผลลัพธ์ (ถ้าไม่ระบุจะแสดงผลทางหน้าจอ)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// สแกนไดเรกทอรีเพื่อหาไฟล์ registry และตรวจสอบความถูกต้อง
    Scan {
        /// ไดเรกทอรีที่ต้องการสแกน
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,

        /// รูปแบบ Glob สำหรับละเว้นไฟล์ (คั่นด้วยจุลภาค) เช่น "**/.git/**,**/node_modules/**"
        #[arg(short, long)]
        ignore: Option<String>,
    },
}

fn main() {
    // ตั้งค่าระบบบันทึก Log
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    if let Err(e) = run() {
        error!("❌ เกิดข้อผิดพลาด: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // จัดการคำสั่งที่ไม่จำเป็นต้องโหลดไฟล์ registry หลักก่อน
    match &cli.command {
        Commands::SchemaExport => {
            let schema = schemars::schema_for!(bl1nk_keyword_core::schema::KeywordRegistry);
            println!("{}", serde_json::to_string_pretty(&schema)?);
            return Ok(());
        }
        Commands::Scan { dir, ignore } => {
            return handle_scan(dir, ignore.clone(), cli.config.clone());
        }
        _ => {}
    }

    // โหลดข้อมูล Registry จากไฟล์
    let mut registry = load_registry(&cli.schema)
        .map_err(|e| anyhow::anyhow!("ไม่สามารถโหลด Registry ได้: {}", e))?;

    // นำกฎการตรวจสอบเพิ่มเติมมาใช้หากมีการระบุ
    if let Some(config_path) = &cli.config {
        let custom_registry = load_registry(config_path)
            .map_err(|e| anyhow::anyhow!("ไม่สามารถโหลดค่ากำหนดเพิ่มเติมได้: {}", e))?;
        registry.validation = custom_registry.validation;
        info!("🔧 โหลดกฎการตรวจสอบเพิ่มเติมจาก {:?}", config_path);
    }

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

        Commands::DocsGen { output } => {
            cmd_docs_gen(&registry, output)?;
        }

        Commands::SchemaExport | Commands::Scan { .. } => unreachable!(),
    }

    Ok(())
}

/// สแกนหาไฟล์ Registry ในไดเรกทอรีและตรวจสอบความถูกต้อง
fn handle_scan(
    dir: &PathBuf,
    ignore: Option<String>,
    config: Option<PathBuf>,
) -> Result<()> {
    info!("🔍 กำลังสแกนไดเรกทอรี: {:?}", dir);

    let ignore_patterns: Vec<String> = if let Some(ig) = ignore {
        ig.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        // รูปแบบการละเว้นไฟล์เริ่มต้น
        vec![
            "**/.git/**".to_string(),
            "**/node_modules/**".to_string(),
            "**/target/**".to_string(),
            "**/dist/**".to_string(),
        ]
    };

    // เตรียมกฎการตรวจสอบเพิ่มเติม
    let mut custom_rules = None;
    if let Some(config_path) = config {
        let custom_registry = load_registry(&config_path)
            .context("ไม่สามารถโหลดไฟล์ค่ากำหนดสำหรับการสแกน")?;
        custom_rules = Some(custom_registry.validation);
    }

    let search_str = dir.join("**/*.{json,yaml,yml}").to_string_lossy().to_string();

    let mut scanned_count = 0;
    let mut valid_count = 0;
    let mut error_count = 0;

    for entry in glob::glob(&search_str).context("รูปแบบ Glob ไม่ถูกต้อง")? {
        match entry {
            Ok(path) => {
                let path_str = path.to_string_lossy().replace("\\", "/");

                // ตรวจสอบว่าไฟล์อยู่ในรายการละเว้นหรือไม่
                let mut is_ignored = false;
                for pattern in &ignore_patterns {
                    if let Ok(matcher) = glob::Pattern::new(pattern) {
                        if matcher.matches(&path_str) {
                            is_ignored = true;
                            break;
                        }
                    }
                }

                if is_ignored {
                    continue;
                }

                scanned_count += 1;
                
                let mut registry = match load_registry(&path) {
                    Ok(r) => r,
                    Err(_) => continue, // ข้ามไฟล์ที่ไม่ใช่รูปแบบ Registry
                };

                // เขียนทับกฎด้วยค่ากำหนดเพิ่มเติมถ้ามี
                if let Some(ref custom_validation) = custom_rules {
                    registry.validation = custom_validation.clone();
                }

                let validator = Validator::new(registry);
                match validator.validate_registry() {
                    Ok(_) => {
                        info!("✅ ตรวจสอบ {}... ผ่าน", path_str);
                        valid_count += 1;
                    }
                    Err(errors) => {
                        warn!("❌ ตรวจสอบ {}... ล้มเหลว", path_str);
                        for err in errors {
                            warn!("  - [{}] {}: {}", err.code, err.field.unwrap_or_default(), err.message);
                        }
                        error_count += 1;
                    }
                }
            }
            Err(e) => error!("❌ เกิดข้อผิดพลาดในการเข้าถึงพาธ: {:?}", e),
        }
    }

    info!("\n--- สรุปการสแกน ---");
    info!("ไฟล์ที่พบและสแกน: {}", scanned_count);
    info!("ไฟล์ที่ถูกต้อง: {}", valid_count);
    
    if error_count > 0 {
        anyhow::bail!("พบไฟล์ที่ไม่ถูกต้องทั้งหมด {} ไฟล์", error_count);
    } else {
        info!("🎉 ไฟล์ที่สแกนทั้งหมดมีความถูกต้อง!");
    }

    Ok(())
}

fn cmd_validate(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    entry_id: Option<String>,
    group: Option<String>,
) -> Result<()> {
    let validator = Validator::new(registry.clone());

    if let Some(id) = entry_id {
        let g_id = group.context("ต้องระบุ Group ID เมื่อตรวจสอบรายการเดียว")?;
        let group_data = registry
            .groups
            .iter()
            .find(|g| g.group_id == g_id)
            .with_context(|| format!("ไม่พบกลุ่ม '{}'", g_id))?;

        let entry = group_data
            .entries
            .iter()
            .find(|e| e.get("id").and_then(|v| v.as_str()) == Some(&id))
            .with_context(|| format!("ไม่พบรายการ '{}' ในกลุ่ม '{}'", id, g_id))?;

        match validator.validate_entry(&g_id, entry) {
            Ok(_) => {
                println!(
                    "{}",
                    json!({ "valid": true, "message": format!("รายการ '{}' มีความถูกต้อง", id) })
                );
            }
            Err(errors) => {
                println!("{}", json!({ "valid": false, "errors": errors }));
                std::process::exit(1);
            }
        }
    } else {
        match validator.validate_registry() {
            Ok(_) => {
                println!(
                    "{}",
                    json!({ "valid": true, "message": "ข้อมูลทั้งหมดมีความถูกต้อง" })
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
) -> Result<()> {
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
            info!("ไม่พบผลลัพธ์สำหรับ: {}", query);
        } else {
            info!("พบผลลัพธ์ {} รายการ:\n", results.len());
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
) -> Result<()> {
    let mut registry = registry.clone();
    let new_entry: Value = if let Some(path) = entry_str.strip_prefix('@') {
        let content = std::fs::read_to_string(path.trim())
            .with_context(|| format!("ไม่สามารถอ่านไฟล์: {}", path))?;
        serde_json::from_str(&content).context("ไฟล์ JSON ไม่ถูกต้อง")?
    } else {
        serde_json::from_str(entry_str).context("รูปแบบ JSON ไม่ถูกต้อง")?
    };

    let validator = Validator::new(registry.clone());

    // 1. ตรวจสอบความถูกต้องของข้อมูลใหม่
    validator.validate_entry(group, &new_entry).map_err(|e| {
        anyhow::anyhow!(
            "การตรวจสอบล้มเหลว: {}",
            serde_json::to_string_pretty(&e).unwrap()
        )
    })?;

    // 2. ตรวจสอบชื่อแฝง (Aliases) ที่ซ้ำกัน
    if let Some(aliases) = new_entry.get("aliases").and_then(|v| v.as_array()) {
        let alias_strings: Vec<String> = aliases
            .iter()
            .filter_map(|a| a.as_str().map(String::from))
            .collect();

        let dup_errors = validator.check_duplicate_aliases(group, None, &alias_strings);
        if !dup_errors.is_empty() {
            anyhow::bail!(
                "พบชื่อแฝงที่ซ้ำกัน: {}",
                serde_json::to_string_pretty(&dup_errors).unwrap()
            );
        }
    }

    // 3. เพิ่มข้อมูลเข้าไปในกลุ่ม
    let group_data = registry
        .groups
        .iter_mut()
        .find(|g| g.group_id == group)
        .with_context(|| format!("ไม่พบกลุ่ม '{}'", group))?;

    group_data.entries.push(new_entry);

    // 4. บันทึกข้อมูลกลับลงไฟล์
    save_registry(schema_path, &registry).context("ไม่สามารถบันทึก Registry ได้")?;
    info!("✅ เพิ่มรายการใหม่สำเร็จ");

    Ok(())
}

fn cmd_edit(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    schema_path: &std::path::PathBuf,
    id: &str,
    group: &str,
    field: &str,
    value: &str,
) -> Result<()> {
    let mut registry = registry.clone();
    let validator = Validator::new(registry.clone());

    let group_data = registry
        .groups
        .iter_mut()
        .find(|g| g.group_id == group)
        .with_context(|| format!("ไม่พบกลุ่ม '{}'", group))?;

    let entry = group_data
        .entries
        .iter_mut()
        .find(|e| e.get("id").and_then(|v| v.as_str()) == Some(id))
        .with_context(|| format!("ไม่พบรายการ '{}' ในกลุ่ม '{}'", id, group))?;

    // อัปเดตค่า (จัดการประเภทข้อมูลเบื้องต้น: JSON หรือ String)
    let new_val: Value = if (value.starts_with('[') && value.ends_with(']')) || (value.starts_with('{') && value.ends_with('}')) {
        serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()))
    } else {
        Value::String(value.to_string())
    };

    entry[field] = new_val;

    // ตรวจสอบความถูกต้องหลังจากแก้ไข
    validator.validate_entry(group, entry).map_err(|e| {
        anyhow::anyhow!(
            "การตรวจสอบล้มเหลวหลังจากแก้ไข: {}",
            serde_json::to_string_pretty(&e).unwrap()
        )
    })?;

    // ตรวจสอบชื่อแฝงหากมีการแก้ไขฟิลด์ aliases
    if field == "aliases" {
        if let Some(aliases) = entry.get("aliases").and_then(|v| v.as_array()) {
            let alias_strings: Vec<String> = aliases
                .iter()
                .filter_map(|a| a.as_str().map(String::from))
                .collect();

            let dup_errors = validator.check_duplicate_aliases(group, Some(id), &alias_strings);
            if !dup_errors.is_empty() {
                anyhow::bail!(
                    "พบชื่อแฝงที่ซ้ำกันหลังจากแก้ไข: {}",
                    serde_json::to_string_pretty(&dup_errors).unwrap()
                );
            }
        }
    }

    save_registry(schema_path, &registry).context("ไม่สามารถบันทึก Registry ได้")?;
    info!("✅ อัปเดตข้อมูลรายการสำเร็จ");

    Ok(())
}

fn cmd_show(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    id: &str,
    json_output: bool,
) -> Result<()> {
    for group in &registry.groups {
        if let Some(entry) = group
            .entries
            .iter()
            .find(|e| e.get("id").and_then(|v| v.as_str()) == Some(id))
        {
            if json_output {
                println!("{}", serde_json::to_string_pretty(entry)?);
            } else {
                info!("--- รายละเอียดรายการ ---");
                println!("ID: {}", id);
                println!("Group: {}", group.group_id);
                println!("{:#}", entry);
            }
            return Ok(());
        }
    }

    anyhow::bail!("ไม่พบรายการ '{}' ในกลุ่มใดๆ", id)
}

fn cmd_list(
    registry: &bl1nk_keyword_core::KeywordRegistry,
    group_id: &str,
    json_output: bool,
) -> Result<()> {
    let group = registry
        .groups
        .iter()
        .find(|g| g.group_id == group_id)
        .with_context(|| format!("ไม่พบกลุ่ม '{}'", group_id))?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&group.entries)?);
    } else {
        info!("รายการข้อมูลในกลุ่ม '{}':", group_id);
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
) -> Result<()> {
    let markdown = bl1nk_keyword_core::generate_markdown(registry);

    match output {
        Some(path) => {
            std::fs::write(&path, markdown).context("ไม่สามารถสร้างไฟล์เอกสารได้")?;
            info!("✅ สร้างเอกสารสำเร็จที่: {}", path.display());
        }
        None => {
            println!("{}", markdown);
        }
    }

    Ok(())
}
