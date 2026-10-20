//! Maestro flow discovery.
//!
//! A Maestro suite is a directory of YAML flows, with optional subflows in
//! a `subflows` directory. The configuration files are not flows, so they
//! are left out.

use std::path::{Path, PathBuf};

/// The files in a Maestro directory that are configuration, not flows.
const CONFIG_FILES: &[&str] = &["config.yaml", "config.yml", "maestro.yaml", "maestro.yml"];

/// Every flow in a Maestro directory, and in its `subflows` directory.
#[must_use]
pub fn discover(maestro_dir: &Path) -> Vec<PathBuf> {
    let mut flows = Vec::new();
    collect(maestro_dir, &mut flows);
    collect(&maestro_dir.join("subflows"), &mut flows);
    flows.sort();
    flows
}

/// The flows in one directory.
fn collect(dir: &Path, flows: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if CONFIG_FILES.contains(&name) {
            continue;
        }
        if name.ends_with(".yaml") || name.ends_with(".yml") {
            flows.push(path);
        }
    }
}

/// Whether a path is a flow file.
#[must_use]
pub fn is_flow(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if CONFIG_FILES.contains(&name) {
        return false;
    }
    name.ends_with(".yaml") || name.ends_with(".yml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_finds_flows_and_skips_configuration() {
        let dir = tempfile::tempdir().expect("temp dir");
        let maestro = dir.path().join(".maestro");
        std::fs::create_dir_all(maestro.join("subflows")).expect("create");

        std::fs::write(maestro.join("01_onboarding.yaml"), "appId: x\n").expect("write");
        std::fs::write(maestro.join("config.yaml"), "appId: x\n").expect("write");
        std::fs::write(maestro.join("notes.txt"), "hello").expect("write");
        std::fs::write(maestro.join("subflows/login.yml"), "appId: x\n").expect("write");

        let flows = discover(&maestro);
        let names: Vec<String> = flows
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["01_onboarding.yaml", "login.yml"]);
    }

    #[test]
    fn test_discover_of_a_missing_directory_is_empty() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert!(discover(&dir.path().join("nope")).is_empty());
    }

    #[test]
    fn test_is_flow() {
        assert!(is_flow(Path::new("a.yaml")));
        assert!(is_flow(Path::new("a.yml")));
        assert!(!is_flow(Path::new("config.yaml")));
        assert!(!is_flow(Path::new("readme.md")));
    }
}
