use bl1nk_keyword_core::load_registry;

fn main() {
    // Test v0.2.0
    let reg = load_registry("schema/keyword-registry-schema-v0.2.0.json")
        .expect("Failed to load v0.2.0");
    println!("v0.2.0: groups={}, version={}", reg.groups.len(), reg.version);
    
    // Test v0.3.0
    let reg = load_registry("schema/keyword-registry-schema-v0.3.0.json")
        .expect("Failed to load v0.3.0");
    println!("v0.3.0: groups={}, version={}, language_mapping={:?}, detection_system={:?}", 
        reg.groups.len(), reg.version, reg.language_mapping.is_some(), reg.detection_system.is_some());
}