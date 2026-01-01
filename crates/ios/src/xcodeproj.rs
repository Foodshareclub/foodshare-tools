//! Xcode project file manipulation
//!
//! Provides tools for reading and modifying .xcodeproj/project.pbxproj files.
//! This is a Rust implementation of common xcodeproj gem operations.

use foodshare_core::error::{Error, Result};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Represents an Xcode project
#[derive(Debug)]
pub struct XcodeProject {
    pub path: PathBuf,
    pub project_dir: PathBuf,
    content: String,
    objects: HashMap<String, PBXObject>,
    root_object_id: String,
}

/// A generic PBX object from the project file
#[derive(Debug, Clone)]
pub struct PBXObject {
    pub id: String,
    pub isa: String,
    pub properties: HashMap<String, String>,
    pub raw: String,
}

/// File reference in the project
#[derive(Debug, Clone)]
pub struct FileReference {
    pub id: String,
    pub path: String,
    pub name: Option<String>,
    pub source_tree: String,
    pub file_type: Option<String>,
}

/// Build file reference
#[derive(Debug, Clone)]
pub struct BuildFile {
    pub id: String,
    pub file_ref_id: String,
}

/// Target in the project
#[derive(Debug, Clone)]
pub struct Target {
    pub id: String,
    pub name: String,
    pub build_phases: Vec<String>,
}

impl XcodeProject {
    /// Open an Xcode project
    pub fn open(path: &Path) -> Result<Self> {
        let pbxproj_path = path.join("project.pbxproj");
        if !pbxproj_path.exists() {
            return Err(Error::Other(format!(
                "project.pbxproj not found at {}",
                pbxproj_path.display()
            )));
        }

        let content = fs::read_to_string(&pbxproj_path)?;
        let project_dir = path
            .parent()
            .ok_or_else(|| Error::Other("Invalid project path".to_string()))?
            .to_path_buf();

        let mut project = Self {
            path: path.to_path_buf(),
            project_dir,
            content: content.clone(),
            objects: HashMap::new(),
            root_object_id: String::new(),
        };

        project.parse()?;
        Ok(project)
    }

    /// Parse the project file
    fn parse(&mut self) -> Result<()> {
        // Extract rootObject
        let root_re = Regex::new(r#"rootObject\s*=\s*([A-F0-9]{24})"#).unwrap();
        if let Some(cap) = root_re.captures(&self.content) {
            self.root_object_id = cap[1].to_string();
        }

        // Parse PBXFileReference entries
        let file_ref_re = Regex::new(
            r#"([A-F0-9]{24})\s*/\*[^*]*\*/\s*=\s*\{isa\s*=\s*PBXFileReference;[^}]+\}"#
        ).unwrap();
        
        for cap in file_ref_re.captures_iter(&self.content) {
            let id = cap[0].split_whitespace().next().unwrap_or("").to_string();
            let body = &cap[0];
            
            let mut properties = HashMap::new();
            properties.insert("isa".to_string(), "PBXFileReference".to_string());
            
            // Extract path
            let path_re = Regex::new(r#"path\s*=\s*"?([^";]+)"?"#).unwrap();
            if let Some(path_cap) = path_re.captures(body) {
                properties.insert("path".to_string(), path_cap[1].trim().to_string());
            }
            
            // Extract name
            let name_re = Regex::new(r#"name\s*=\s*"?([^";]+)"?"#).unwrap();
            if let Some(name_cap) = name_re.captures(body) {
                properties.insert("name".to_string(), name_cap[1].trim().to_string());
            }
            
            // Extract sourceTree
            let tree_re = Regex::new(r#"sourceTree\s*=\s*"?([^";]+)"?"#).unwrap();
            if let Some(tree_cap) = tree_re.captures(body) {
                properties.insert("sourceTree".to_string(), tree_cap[1].trim().to_string());
            }
            
            self.objects.insert(
                id.clone(),
                PBXObject {
                    id,
                    isa: "PBXFileReference".to_string(),
                    properties,
                    raw: body.to_string(),
                },
            );
        }

        // Parse PBXBuildFile entries
        let build_file_re = Regex::new(
            r#"([A-F0-9]{24})\s*/\*[^*]*\*/\s*=\s*\{isa\s*=\s*PBXBuildFile;\s*fileRef\s*=\s*([A-F0-9]{24})"#
        ).unwrap();
        
        for cap in build_file_re.captures_iter(&self.content) {
            let id = cap[1].to_string();
            let file_ref = cap[2].to_string();
            
            let mut properties = HashMap::new();
            properties.insert("isa".to_string(), "PBXBuildFile".to_string());
            properties.insert("fileRef".to_string(), file_ref);
            
            self.objects.insert(
                id.clone(),
                PBXObject {
                    id,
                    isa: "PBXBuildFile".to_string(),
                    properties,
                    raw: cap[0].to_string(),
                },
            );
        }

        // Parse PBXNativeTarget entries
        let target_re = Regex::new(
            r#"([A-F0-9]{24})\s*/\*\s*([^*]+)\s*\*/\s*=\s*\{[^}]*isa\s*=\s*PBXNativeTarget"#
        ).unwrap();
        
        for cap in target_re.captures_iter(&self.content) {
            let id = cap[1].to_string();
            let name = cap[2].trim().to_string();
            
            let mut properties = HashMap::new();
            properties.insert("isa".to_string(), "PBXNativeTarget".to_string());
            properties.insert("name".to_string(), name);
            
            // Find buildPhases for this target - search in a larger context
            let target_start = cap.get(0).unwrap().start();
            let search_end = (target_start + 2000).min(self.content.len());
            let target_context = &self.content[target_start..search_end];
            
            let phases_re = Regex::new(r#"buildPhases\s*=\s*\(([^)]+)\)"#).unwrap();
            if let Some(phases_cap) = phases_re.captures(target_context) {
                properties.insert("buildPhases".to_string(), phases_cap[1].to_string());
            }
            
            self.objects.insert(
                id.clone(),
                PBXObject {
                    id,
                    isa: "PBXNativeTarget".to_string(),
                    properties,
                    raw: String::new(),
                },
            );
        }

        // Parse PBXSourcesBuildPhase entries
        let sources_re = Regex::new(
            r#"([A-F0-9]{24})\s*/\*[^*]*\*/\s*=\s*\{[^}]*isa\s*=\s*PBXSourcesBuildPhase"#
        ).unwrap();
        
        for cap in sources_re.captures_iter(&self.content) {
            let id = cap[1].to_string();
            
            let mut properties = HashMap::new();
            properties.insert("isa".to_string(), "PBXSourcesBuildPhase".to_string());
            
            // Find files for this phase
            let phase_start = cap.get(0).unwrap().start();
            let search_end = (phase_start + 50000).min(self.content.len());
            let phase_context = &self.content[phase_start..search_end];
            
            let files_re = Regex::new(r#"files\s*=\s*\(([^)]+)\)"#).unwrap();
            if let Some(files_cap) = files_re.captures(phase_context) {
                properties.insert("files".to_string(), files_cap[1].to_string());
            }
            
            self.objects.insert(
                id.clone(),
                PBXObject {
                    id,
                    isa: "PBXSourcesBuildPhase".to_string(),
                    properties,
                    raw: String::new(),
                },
            );
        }

        Ok(())
    }

    /// Get all file references
    pub fn file_references(&self) -> Vec<FileReference> {
        self.objects
            .values()
            .filter(|obj| obj.isa == "PBXFileReference")
            .map(|obj| FileReference {
                id: obj.id.clone(),
                path: obj.properties.get("path").cloned().unwrap_or_default(),
                name: obj.properties.get("name").cloned(),
                source_tree: obj
                    .properties
                    .get("sourceTree")
                    .cloned()
                    .unwrap_or_else(|| "<group>".to_string()),
                file_type: obj.properties.get("lastKnownFileType").cloned(),
            })
            .collect()
    }

    /// Get all targets
    pub fn targets(&self) -> Vec<Target> {
        self.objects
            .values()
            .filter(|obj| obj.isa == "PBXNativeTarget")
            .map(|obj| {
                let name = obj.properties.get("name").cloned().unwrap_or_default();
                // Parse buildPhases array
                let phases_re = Regex::new(r#"([A-F0-9]{24})"#).unwrap();
                let phases_str = obj.properties.get("buildPhases").cloned().unwrap_or_default();
                let build_phases: Vec<String> = phases_re
                    .captures_iter(&phases_str)
                    .map(|c| c[1].to_string())
                    .collect();

                Target {
                    id: obj.id.clone(),
                    name,
                    build_phases,
                }
            })
            .collect()
    }

    /// Find target by name
    pub fn find_target(&self, name: &str) -> Option<Target> {
        self.targets().into_iter().find(|t| t.name == name)
    }

    /// Get build files for a source build phase
    pub fn build_files_for_phase(&self, phase_id: &str) -> Vec<BuildFile> {
        if let Some(phase) = self.objects.get(phase_id) {
            if phase.isa == "PBXSourcesBuildPhase" {
                let files_re = Regex::new(r#"([A-F0-9]{24})"#).unwrap();
                let files_str = phase.properties.get("files").cloned().unwrap_or_default();
                return files_re
                    .captures_iter(&files_str)
                    .filter_map(|c| {
                        let bf_id = c[1].to_string();
                        self.objects.get(&bf_id).map(|bf| BuildFile {
                            id: bf_id,
                            file_ref_id: bf
                                .properties
                                .get("fileRef")
                                .cloned()
                                .unwrap_or_default(),
                        })
                    })
                    .collect();
            }
        }
        Vec::new()
    }

    /// Get all Swift files in the build phase for a target
    pub fn swift_files_in_build(&self, target_name: &str) -> Vec<PathBuf> {
        let target = match self.find_target(target_name) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut files = Vec::new();
        for phase_id in &target.build_phases {
            for build_file in self.build_files_for_phase(phase_id) {
                if let Some(file_ref) = self.objects.get(&build_file.file_ref_id) {
                    if let Some(path) = file_ref.properties.get("path") {
                        if path.ends_with(".swift") {
                            files.push(PathBuf::from(path));
                        }
                    }
                }
            }
        }
        files
    }

    /// Find all Swift files on disk in a directory
    pub fn find_swift_files_on_disk(&self, source_dir: &str) -> Result<Vec<PathBuf>> {
        let dir = self.project_dir.join(source_dir);
        let mut files = Vec::new();

        if dir.exists() {
            Self::find_swift_files_recursive(&dir, &mut files)?;
        }

        Ok(files)
    }

    fn find_swift_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip test directories
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.contains("Tests") {
                    Self::find_swift_files_recursive(&path, files)?;
                }
            } else if path.extension().map_or(false, |e| e == "swift") {
                files.push(path);
            }
        }
        Ok(())
    }

    /// Find missing files (on disk but not in build phase)
    pub fn find_missing_files(&self, target_name: &str, source_dir: &str) -> Result<Vec<PathBuf>> {
        let disk_files = self.find_swift_files_on_disk(source_dir)?;
        let build_files: HashSet<_> = self
            .swift_files_in_build(target_name)
            .into_iter()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
            .collect();

        let missing: Vec<PathBuf> = disk_files
            .into_iter()
            .filter(|p| {
                let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                !build_files.contains(&name)
            })
            .collect();

        Ok(missing)
    }

    /// Find broken references (in project but file doesn't exist)
    pub fn find_broken_references(&self) -> Vec<FileReference> {
        self.file_references()
            .into_iter()
            .filter(|fr| {
                if fr.path.is_empty() {
                    return false;
                }
                let full_path = if fr.path.starts_with('/') {
                    PathBuf::from(&fr.path)
                } else {
                    self.project_dir.join(&fr.path)
                };
                !full_path.exists()
            })
            .collect()
    }

    /// Find duplicate build file references
    pub fn find_duplicate_build_files(&self, target_name: &str) -> Vec<(String, Vec<BuildFile>)> {
        let target = match self.find_target(target_name) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut file_refs: HashMap<String, Vec<BuildFile>> = HashMap::new();

        for phase_id in &target.build_phases {
            for build_file in self.build_files_for_phase(phase_id) {
                file_refs
                    .entry(build_file.file_ref_id.clone())
                    .or_default()
                    .push(build_file);
            }
        }

        file_refs
            .into_iter()
            .filter(|(_, bfs)| bfs.len() > 1)
            .collect()
    }

    /// Get project status summary
    pub fn status(&self, target_name: &str, source_dir: &str) -> Result<ProjectStatus> {
        let missing = self.find_missing_files(target_name, source_dir)?;
        let broken = self.find_broken_references();
        let duplicates = self.find_duplicate_build_files(target_name);
        let total_files = self.swift_files_in_build(target_name).len();

        Ok(ProjectStatus {
            total_build_files: total_files,
            missing_files: missing.len(),
            broken_references: broken.len(),
            duplicate_references: duplicates.len(),
            missing_file_paths: missing,
            broken_reference_paths: broken.into_iter().map(|fr| fr.path).collect(),
        })
    }
}

/// Project status summary
#[derive(Debug)]
pub struct ProjectStatus {
    pub total_build_files: usize,
    pub missing_files: usize,
    pub broken_references: usize,
    pub duplicate_references: usize,
    pub missing_file_paths: Vec<PathBuf>,
    pub broken_reference_paths: Vec<String>,
}

impl ProjectStatus {
    pub fn is_clean(&self) -> bool {
        self.missing_files == 0 && self.broken_references == 0 && self.duplicate_references == 0
    }

    pub fn print(&self) {
        use owo_colors::OwoColorize;

        println!("{}", "Xcode Project Status".bold());
        println!();
        println!("  Total build files: {}", self.total_build_files);

        if self.missing_files > 0 {
            println!(
                "  {} Missing files: {}",
                "⚠".yellow(),
                self.missing_files
            );
        } else {
            println!("  {} No missing files", "✓".green());
        }

        if self.broken_references > 0 {
            println!(
                "  {} Broken references: {}",
                "⚠".yellow(),
                self.broken_references
            );
        } else {
            println!("  {} No broken references", "✓".green());
        }

        if self.duplicate_references > 0 {
            println!(
                "  {} Duplicate references: {}",
                "⚠".yellow(),
                self.duplicate_references
            );
        } else {
            println!("  {} No duplicates", "✓".green());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_status_is_clean() {
        let status = ProjectStatus {
            total_build_files: 100,
            missing_files: 0,
            broken_references: 0,
            duplicate_references: 0,
            missing_file_paths: Vec::new(),
            broken_reference_paths: Vec::new(),
        };
        assert!(status.is_clean());
    }

    #[test]
    fn test_project_status_not_clean() {
        let status = ProjectStatus {
            total_build_files: 100,
            missing_files: 5,
            broken_references: 0,
            duplicate_references: 0,
            missing_file_paths: Vec::new(),
            broken_reference_paths: Vec::new(),
        };
        assert!(!status.is_clean());
    }
}
