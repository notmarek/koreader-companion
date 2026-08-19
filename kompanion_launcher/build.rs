use std::env;
use std::fs;
use std::path::Path;

#[path = "build_utils.rs"]
mod build_utils;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_dir = Path::new(&manifest_dir);
    let workspace_toml = manifest_dir.parent().unwrap().join("Cargo.toml");
    let toml_content = fs::read_to_string(&workspace_toml)
        .expect("failed to read workspace Cargo.toml");

    let kpm_dir = manifest_dir.parent().unwrap().join("kpm");
    let manifest_path = kpm_dir.join("manifest.json");

    let name = env::var("CARGO_PKG_NAME").unwrap();
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let authors = env::var("CARGO_PKG_AUTHORS").unwrap();
    let description = env::var("CARGO_PKG_DESCRIPTION").unwrap();

    let package_id = extract_toml_value(&toml_content, "id")
        .unwrap_or_else(|| name.clone());
    let package_name = extract_toml_value(&toml_content, "name")
        .unwrap_or_else(|| name.clone());

    let version_array: Vec<u32> = version
        .split('.')
        .map(|v| v.parse().expect("invalid semver"))
        .collect();
    assert!(version_array.len() == 3, "version must be semver");

    let mf_version = extract_toml_value(&toml_content, "manifest_version")
        .unwrap_or_else(|| "2".to_string());
    let platforms = extract_toml_array(&toml_content, "supported_platforms")
        .unwrap_or_else(|| vec!["kindlehf".to_string(), "kindlepw2".to_string()]);

    let platforms_json: String = platforms
        .iter()
        .map(|p| format!("\"{}\"", p))
        .collect::<Vec<_>>()
        .join(", ");

    let json = format!(
        r#"{{
  "manifest_version": {},
  "id": "{}",
  "name": "{}",
  "author": "{}",
  "description": "{}",
  "version": [
    {},
    {},
    {}
  ],
  "dependencies": [],
  "supported_platforms": [{}]
}}
"#,
        mf_version,
        package_id,
        package_name,
        authors,
        description,
        version_array[0],
        version_array[1],
        version_array[2],
        platforms_json,
    );

    fs::create_dir_all(&kpm_dir).ok();
    let prev = fs::read_to_string(&manifest_path).unwrap_or_default();

    if prev != json {
        fs::write(&manifest_path, &json).expect("failed to write manifest.json");
        println!("cargo:warning=generated kpm/manifest.json");
    }

    let indexer_dir = manifest_dir.parent().unwrap().join("kompanion_extractor/src/indexer");
    let specs = build_utils::parse_indexer_specs(&indexer_dir);

    let install_sql = build_utils::generate_install_sql(&specs);
    let install_path = kpm_dir.join("install.sql");
    let prev_install = fs::read_to_string(&install_path).unwrap_or_default();
    if prev_install != install_sql {
        fs::write(&install_path, &install_sql).expect("failed to write install.sql");
        println!("cargo:warning=generated kpm/install.sql");
    }

    let uninstall_sql = build_utils::generate_uninstall_sql(&specs);
    let uninstall_path = kpm_dir.join("uninstall.sql");
    let prev_uninstall = fs::read_to_string(&uninstall_path).unwrap_or_default();
    if prev_uninstall != uninstall_sql {
        fs::write(&uninstall_path, &uninstall_sql).expect("failed to write uninstall.sql");
        println!("cargo:warning=generated kpm/uninstall.sql");
    }

    println!("cargo:rerun-if-changed=../Cargo.toml");
    println!("cargo:rerun-if-changed=../kompanion_extractor/src/indexer");
    println!("cargo:rerun-if-changed=build_utils.rs");
    println!("cargo:rerun-if-changed=build.rs");
}

fn extract_toml_value(toml: &str, key: &str) -> Option<String> {
    let prefix = format!("{} = ", key);
    for line in toml.lines() {
        let trimmed = line.trim();
        let trimmed = trimmed.split('#').next().unwrap_or("").trim();
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let val = rest.trim().trim_matches('"');
            return Some(val.to_string());
        }
    }
    None
}

fn extract_toml_array(toml: &str, key: &str) -> Option<Vec<String>> {
    let prefix = format!("{} = [", key);
    for line in toml.lines() {
        let trimmed = line.trim();
        let trimmed = trimmed.split('#').next().unwrap_or("").trim();
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let items_str = rest.trim_end_matches(']');
            let items: Vec<String> = items_str
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return Some(items);
        }
    }
    None
}
