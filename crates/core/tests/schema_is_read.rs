//! Every key the schema declares must be read by something.
//!
//! The report of the keys a tool does not implement compares the file
//! with the schema. A key that the schema declares but that nothing
//! reads is invisible to that report: the report would say the key is
//! implemented, and it would not be. A repository that writes it would
//! believe it had an effect.
//!
//! This test is what keeps the schema and the implementation in step. It
//! reads the schema's own source for the keys it declares, and looks
//! for a reader of each one in the rest of the workspace.
//!
//! A reader is a file that mentions the section and accesses the field:
//! `config.schema.general.scheme`, or `config.max_length` for a section
//! that is passed around whole. The test cannot tell which struct a
//! field belongs to, so a field that shares its name with another field
//! elsewhere counts as read. That makes the test weaker than perfect
//! and much stronger than nothing: a field nothing touches is still
//! caught.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The workspace root, from this crate's manifest.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate should be inside the workspace")
        .to_path_buf()
}

/// `GeneralConfig` is the `general` section; `CommitMsgConfig` is
/// `commit_msg`.
fn section_name(struct_name: &str) -> String {
    let stem = struct_name.strip_suffix("Config").unwrap_or(struct_name);
    let mut snake = String::new();
    for (index, character) in stem.chars().enumerate() {
        if character.is_ascii_uppercase() && index > 0 {
            snake.push('_');
        }
        snake.push(character.to_ascii_lowercase());
    }
    snake
}

/// Every key the schema declares: the sections, and the fields in them.
fn declared_keys() -> Vec<(String, String)> {
    let schema = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/config/schema.rs");
    let source = std::fs::read_to_string(&schema).expect("the schema should be readable");

    let mut keys = Vec::new();
    let mut section = String::new();
    let mut in_root = false;

    for line in source.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("pub struct ") {
            let name = rest.split('{').next().unwrap_or(rest).trim();
            in_root = name == "ConfigSchema";
            section = if in_root {
                String::new()
            } else {
                section_name(name)
            };
            continue;
        }

        if let Some(name) = trimmed.strip_prefix("pub ") {
            let name = name.split(':').next().unwrap_or(name).trim();
            if name.is_empty()
                || !name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            {
                continue;
            }

            if in_root {
                // A field of the root is a section.
                keys.push((String::new(), name.to_string()));
            } else {
                keys.push((section.clone(), name.to_string()));
            }
        }
    }

    keys
}

/// Every Rust source in the workspace, except the schema itself.
fn sources() -> Vec<PathBuf> {
    let root = workspace_root();
    let mut found = Vec::new();

    for directory in ["crates", "bins"] {
        let base = root.join(directory);
        let Ok(entries) = std::fs::read_dir(&base) else {
            continue;
        };
        for entry in entries.flatten() {
            collect(&entry.path().join("src"), &mut found);
        }
    }

    found
}

/// Every `.rs` file below a directory.
fn collect(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
}

/// The code of a file, with the comments out and the whitespace in.
///
/// The whitespace goes because the formatter breaks a long access across
/// lines — `config\n.schema\n.migrations` is one access, and a search
/// for `.schema.migrations` has to find it.
fn code_of(body: &str) -> String {
    let mut without_block_comments = String::new();
    let mut in_block_comment = false;

    for line in body.lines() {
        let mut rest = line;
        loop {
            if in_block_comment {
                match rest.find("*/") {
                    Some(end) => {
                        rest = &rest[end + 2..];
                        in_block_comment = false;
                    }
                    None => {
                        rest = "";
                        break;
                    }
                }
            } else {
                match rest.find("/*") {
                    Some(start) => {
                        rest = &rest[..start];
                        in_block_comment = true;
                    }
                    None => break,
                }
            }
        }

        // A `//` that is not inside a string starts a comment.
        let code = match rest.find("//") {
            Some(index) if rest[..index].matches('"').count() % 2 == 0 => &rest[..index],
            _ => rest,
        };
        without_block_comments.push_str(code);
        without_block_comments.push('\n');
    }

    without_block_comments
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

#[test]
fn every_declared_key_is_read_by_something() {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/config/schema.rs");
    let sources = sources();

    // The schema's own file is excluded: the keys are declared there,
    // not read there.
    let bodies: Vec<String> = sources
        .iter()
        .filter(|path| *path != &schema_path)
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .map(|body| code_of(&body))
        .collect();

    let mut unread = BTreeSet::new();

    for (section, field) in declared_keys() {
        let read = if section.is_empty() {
            // A section of the root: read as `config.schema.<section>`.
            let accessed = format!(".schema.{field}");
            bodies.iter().any(|body| body.contains(&accessed))
        } else {
            // A field of a section: read as `<something>.<field>`, in a
            // file that mentions the section. A field name on its own is
            // not enough, because the same name is used by other
            // structs: `status.swift_version` is not a reader of the
            // configuration's `general.swift_version`.
            let accessed = format!(".{field}");
            bodies
                .iter()
                .any(|body| body.contains(&section) && body.contains(&accessed))
        };

        if !read {
            let name = if section.is_empty() {
                field
            } else {
                format!("{section}.{field}")
            };
            unread.insert(name);
        }
    }

    assert!(
        unread.is_empty(),
        "these keys are declared in the schema but nothing reads them, so the report \
         of the keys a tool does not implement cannot see them:\n  {}",
        unread.into_iter().collect::<Vec<_>>().join("\n  ")
    );
}
