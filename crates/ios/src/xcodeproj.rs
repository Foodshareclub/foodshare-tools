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

/// File type classification for Xcode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Swift,
    ObjC,
    Metal,
    Header,
    Resource,
    Framework,
    Unknown,
}

impl FileType {
    /// Determine file type from extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "swift" => FileType::Swift,
            "m" | "mm" => FileType::ObjC,
            "metal" => FileType::Metal,
            "h" | "hpp" | "pch" => FileType::Header,
            "xib" | "storyboard" | "xcassets" | "json" | "plist" | "strings" | "png" | "jpg"
            | "jpeg" | "gif" | "pdf" | "ttf" | "otf" => FileType::Resource,
            "framework" | "xcframework" => FileType::Framework,
            _ => FileType::Unknown,
        }
    }

    /// Get Xcode's lastKnownFileType value for this file type
    pub fn last_known_file_type(&self) -> &'static str {
        match self {
            FileType::Swift => "sourcecode.swift",
            FileType::ObjC => "sourcecode.c.objc",
            FileType::Metal => "sourcecode.metal",
            FileType::Header => "sourcecode.c.h",
            FileType::Resource => "file",
            FileType::Framework => "wrapper.framework",
            FileType::Unknown => "file",
        }
    }

    /// Whether this file type should be added to the Sources build phase
    pub fn should_add_to_sources(&self) -> bool {
        matches!(self, FileType::Swift | FileType::ObjC | FileType::Metal)
    }
}

/// Result of adding a file to the project
#[derive(Debug)]
pub struct AddFileResult {
    pub file_ref_id: String,
    pub build_file_id: Option<String>,
    pub group_id: String,
    pub path: PathBuf,
    pub already_exists: bool,
}

/// Reference to a PBXGroup in the project
#[derive(Debug, Clone)]
pub struct GroupReference {
    pub id: String,
    pub name: Option<String>,
    pub path: Option<String>,
    pub children: Vec<String>,
    pub source_tree: String,
}

impl XcodeProject {
    /// Open an Xcode project
    pub fn open(path: &Path) -> Result<Self> {
        let pbxproj_path = path.join("project.pbxproj");
        if !pbxproj_path.exists() {
            return Err(Error::file_not_found(&pbxproj_path));
        }

        let content = fs::read_to_string(&pbxproj_path)?;
        let project_dir = path
            .parent()
            .ok_or_else(|| Error::validation("Invalid project path"))?
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

    // ========================================================================
    // UUID Generation
    // ========================================================================

    /// Generate a 24-character uppercase hex UUID (Xcode format)
    fn generate_uuid(&self) -> String {
        use uuid::Uuid;
        Uuid::new_v4()
            .simple()
            .to_string()
            .to_uppercase()
            .chars()
            .take(24)
            .collect()
    }

    /// Generate a unique UUID not already in the project
    fn generate_unique_uuid(&self) -> String {
        loop {
            let id = self.generate_uuid();
            if !self.objects.contains_key(&id) {
                return id;
            }
        }
    }

    // ========================================================================
    // Group Parsing
    // ========================================================================

    /// Parse all PBXGroup entries from the project file
    fn parse_groups(&self) -> Vec<GroupReference> {
        let mut groups = Vec::new();

        // Match PBXGroup entries
        let group_re = Regex::new(
            r#"([A-F0-9]{24})\s*/\*[^*]*\*/\s*=\s*\{[^}]*isa\s*=\s*PBXGroup"#,
        )
        .unwrap();

        for cap in group_re.captures_iter(&self.content) {
            let id = cap[1].to_string();
            let start = cap.get(0).unwrap().start();

            // Find the full group block - need to find matching closing brace
            let search_end = (start + 5000).min(self.content.len());
            let block = &self.content[start..search_end];

            // Extract name
            let name = Regex::new(r#"name\s*=\s*"?([^";]+)"?"#)
                .ok()
                .and_then(|re| re.captures(block))
                .map(|c| c[1].trim().to_string());

            // Extract path
            let path = Regex::new(r#"(?m)^\s*path\s*=\s*"?([^";]+)"?"#)
                .ok()
                .and_then(|re| re.captures(block))
                .map(|c| c[1].trim().to_string());

            // Extract sourceTree
            let source_tree = Regex::new(r#"sourceTree\s*=\s*"?([^";]+)"?"#)
                .ok()
                .and_then(|re| re.captures(block))
                .map(|c| c[1].trim().to_string())
                .unwrap_or_else(|| "<group>".to_string());

            // Extract children
            let children = Regex::new(r#"children\s*=\s*\(([^)]*)\)"#)
                .ok()
                .and_then(|re| re.captures(block))
                .map(|c| {
                    Regex::new(r#"([A-F0-9]{24})"#)
                        .unwrap()
                        .captures_iter(&c[1])
                        .map(|m| m[1].to_string())
                        .collect()
                })
                .unwrap_or_default();

            groups.push(GroupReference {
                id,
                name,
                path,
                children,
                source_tree,
            });
        }

        groups
    }

    /// Get all groups in the project
    pub fn groups(&self) -> Vec<GroupReference> {
        self.parse_groups()
    }

    /// Find the main (root) group ID from PBXProject
    pub fn find_main_group_id(&self) -> Option<String> {
        let re = Regex::new(r#"mainGroup\s*=\s*([A-F0-9]{24})"#).ok()?;
        re.captures(&self.content).map(|c| c[1].to_string())
    }

    /// Find a group by its path (e.g., "FoodShare/Core/Design")
    pub fn find_group_by_path(&self, path: &str) -> Option<GroupReference> {
        let groups = self.parse_groups();
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if path_parts.is_empty() {
            return self.find_main_group_id().and_then(|id| {
                groups.into_iter().find(|g| g.id == id)
            });
        }

        // Start from main group
        let main_group_id = self.find_main_group_id()?;
        let mut current_group = groups.iter().find(|g| g.id == main_group_id)?;

        for part in &path_parts {
            let mut found = false;
            for child_id in &current_group.children {
                if let Some(child_group) = groups.iter().find(|g| &g.id == child_id) {
                    let matches = child_group.name.as_deref() == Some(*part)
                        || child_group.path.as_deref() == Some(*part);
                    if matches {
                        current_group = child_group;
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                return None;
            }
        }

        Some(current_group.clone())
    }

    // ========================================================================
    // Write Operations
    // ========================================================================

    /// Check if a file path is already referenced in the project
    pub fn file_exists_in_project(&self, file_path: &Path) -> bool {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let path_str = file_path.to_string_lossy();

        self.file_references()
            .iter()
            .any(|fr| fr.path == path_str || fr.path.ends_with(file_name))
    }

    /// Quote a string if it contains special characters
    fn quote_if_needed(s: &str) -> String {
        if s.contains(' ') || s.contains('/') || s.contains('-') || s.contains('.') {
            format!("\"{}\"", s)
        } else {
            s.to_string()
        }
    }

    /// Add a file to the project
    pub fn add_file(
        &mut self,
        file_path: &Path,
        target_name: &str,
        group_path: Option<&str>,
    ) -> Result<AddFileResult> {
        // Verify file exists on disk
        let full_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            self.project_dir.join(file_path)
        };

        if !full_path.exists() {
            return Err(Error::file_not_found(&full_path));
        }

        // Check if already in project
        if self.file_exists_in_project(file_path) {
            let groups = self.parse_groups();
            let group_id = groups.first().map(|g| g.id.clone()).unwrap_or_default();
            return Ok(AddFileResult {
                file_ref_id: String::new(),
                build_file_id: None,
                group_id,
                path: file_path.to_path_buf(),
                already_exists: true,
            });
        }

        // Determine file type
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let file_type = FileType::from_extension(ext);
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Generate IDs
        let file_ref_id = self.generate_unique_uuid();

        // Register the file_ref_id to avoid collisions
        self.objects.insert(
            file_ref_id.clone(),
            PBXObject {
                id: file_ref_id.clone(),
                isa: "PBXFileReference".to_string(),
                properties: HashMap::new(),
                raw: String::new(),
            },
        );

        let build_file_id = if file_type.should_add_to_sources() {
            let id = self.generate_unique_uuid();
            self.objects.insert(
                id.clone(),
                PBXObject {
                    id: id.clone(),
                    isa: "PBXBuildFile".to_string(),
                    properties: HashMap::new(),
                    raw: String::new(),
                },
            );
            Some(id)
        } else {
            None
        };

        // Find or create the target group
        let group_id = self.find_or_create_group(group_path, file_path)?;

        // Add PBXFileReference entry
        self.add_file_reference(&file_ref_id, file_path, &file_type)?;

        // Add file to group's children
        self.add_child_to_group(&group_id, &file_ref_id, file_name)?;

        // If it's a source file, add to build phase
        if let Some(ref bf_id) = build_file_id {
            // Add PBXBuildFile entry
            self.add_build_file(bf_id, &file_ref_id, file_name)?;

            // Find the Sources build phase for the target and add the build file
            if let Some(target) = self.find_target(target_name) {
                for phase_id in &target.build_phases {
                    if let Some(phase) = self.objects.get(phase_id) {
                        if phase.isa == "PBXSourcesBuildPhase" {
                            self.add_file_to_build_phase(phase_id, bf_id, file_name)?;
                            break;
                        }
                    }
                }
            }
        }

        Ok(AddFileResult {
            file_ref_id,
            build_file_id,
            group_id,
            path: file_path.to_path_buf(),
            already_exists: false,
        })
    }

    /// Add a PBXFileReference entry to the project content
    fn add_file_reference(
        &mut self,
        id: &str,
        path: &Path,
        file_type: &FileType,
    ) -> Result<()> {
        let path_str = path.to_string_lossy();
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let last_known = file_type.last_known_file_type();

        // Format the entry
        let entry = format!(
            "\t\t{} /* {} */ = {{isa = PBXFileReference; lastKnownFileType = {}; path = {}; sourceTree = SOURCE_ROOT; }};\n",
            id,
            file_name,
            last_known,
            Self::quote_if_needed(&path_str)
        );

        // Find the end of PBXFileReference section and insert before it
        let marker = "/* End PBXFileReference section */";
        if let Some(pos) = self.content.find(marker) {
            self.content.insert_str(pos, &entry);
        } else {
            return Err(Error::validation(
                "Could not find PBXFileReference section in project file",
            ));
        }

        Ok(())
    }

    /// Add a PBXBuildFile entry to the project content
    fn add_build_file(&mut self, id: &str, file_ref_id: &str, file_name: &str) -> Result<()> {
        let entry = format!(
            "\t\t{} /* {} in Sources */ = {{isa = PBXBuildFile; fileRef = {} /* {} */; }};\n",
            id, file_name, file_ref_id, file_name
        );

        let marker = "/* End PBXBuildFile section */";
        if let Some(pos) = self.content.find(marker) {
            self.content.insert_str(pos, &entry);
        } else {
            return Err(Error::validation(
                "Could not find PBXBuildFile section in project file",
            ));
        }

        Ok(())
    }

    /// Add a build file ID to a build phase's files array
    fn add_file_to_build_phase(
        &mut self,
        phase_id: &str,
        build_file_id: &str,
        file_name: &str,
    ) -> Result<()> {
        // Find the build phase entry
        let phase_pattern = format!(r#"{}\s*/\*[^*]*\*/\s*=\s*\{{[^}}]*files\s*=\s*\("#, phase_id);
        let phase_re = Regex::new(&phase_pattern).map_err(|e| {
            Error::validation(&format!("Invalid regex pattern: {}", e))
        })?;

        if let Some(cap) = phase_re.find(&self.content) {
            let insert_pos = cap.end();
            let entry = format!(
                "\n\t\t\t\t{} /* {} in Sources */,",
                build_file_id, file_name
            );
            self.content.insert_str(insert_pos, &entry);
        }

        Ok(())
    }

    /// Add a child ID to a group's children array
    fn add_child_to_group(&mut self, group_id: &str, child_id: &str, file_name: &str) -> Result<()> {
        // Find the group entry and its children array
        let group_pattern = format!(r#"{}\s*/\*[^*]*\*/\s*=\s*\{{[^}}]*children\s*=\s*\("#, group_id);
        let group_re = Regex::new(&group_pattern).map_err(|e| {
            Error::validation(&format!("Invalid regex pattern: {}", e))
        })?;

        if let Some(cap) = group_re.find(&self.content) {
            let insert_pos = cap.end();
            let entry = format!("\n\t\t\t\t{} /* {} */,", child_id, file_name);
            self.content.insert_str(insert_pos, &entry);
        }

        Ok(())
    }

    /// Find or create a group for the file
    fn find_or_create_group(
        &mut self,
        explicit_group: Option<&str>,
        file_path: &Path,
    ) -> Result<String> {
        // If explicit group path provided, try to find it
        if let Some(group_path) = explicit_group {
            if let Some(group) = self.find_group_by_path(group_path) {
                return Ok(group.id);
            }
            // Group doesn't exist - we could create it, but for now just use parent dir
        }

        // Use the file's parent directory as the group path
        let parent = file_path
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");

        if let Some(group) = self.find_group_by_path(parent) {
            return Ok(group.id);
        }

        // Fall back to main group
        self.find_main_group_id()
            .ok_or_else(|| Error::validation("Could not find main group in project"))
    }

    /// Create a new group in the project
    #[allow(dead_code)]
    fn create_group(&mut self, id: &str, name: &str, path: Option<&str>) -> Result<()> {
        let path_attr = path
            .map(|p| format!("path = {}; ", Self::quote_if_needed(p)))
            .unwrap_or_default();

        let entry = format!(
            "\t\t{} /* {} */ = {{\n\t\t\tisa = PBXGroup;\n\t\t\tchildren = (\n\t\t\t);\n\t\t\t{}name = {};\n\t\t\tsourceTree = \"<group>\";\n\t\t}};\n",
            id,
            name,
            path_attr,
            Self::quote_if_needed(name)
        );

        let marker = "/* End PBXGroup section */";
        if let Some(pos) = self.content.find(marker) {
            self.content.insert_str(pos, &entry);
        } else {
            return Err(Error::validation(
                "Could not find PBXGroup section in project file",
            ));
        }

        Ok(())
    }

    // ========================================================================
    // Save Operations
    // ========================================================================

    /// Save the project file (creates a backup first)
    pub fn save(&self) -> Result<()> {
        let pbxproj_path = self.path.join("project.pbxproj");
        let backup_path = pbxproj_path.with_extension("pbxproj.backup");

        // Create backup
        fs::copy(&pbxproj_path, &backup_path)?;

        // Write updated content
        fs::write(&pbxproj_path, &self.content)?;

        Ok(())
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

    #[test]
    fn test_file_type_from_extension_swift() {
        assert_eq!(FileType::from_extension("swift"), FileType::Swift);
        assert_eq!(FileType::from_extension("SWIFT"), FileType::Swift);
    }

    #[test]
    fn test_file_type_from_extension_objc() {
        assert_eq!(FileType::from_extension("m"), FileType::ObjC);
        assert_eq!(FileType::from_extension("mm"), FileType::ObjC);
    }

    #[test]
    fn test_file_type_from_extension_metal() {
        assert_eq!(FileType::from_extension("metal"), FileType::Metal);
    }

    #[test]
    fn test_file_type_from_extension_header() {
        assert_eq!(FileType::from_extension("h"), FileType::Header);
        assert_eq!(FileType::from_extension("hpp"), FileType::Header);
        assert_eq!(FileType::from_extension("pch"), FileType::Header);
    }

    #[test]
    fn test_file_type_from_extension_resource() {
        assert_eq!(FileType::from_extension("json"), FileType::Resource);
        assert_eq!(FileType::from_extension("plist"), FileType::Resource);
        assert_eq!(FileType::from_extension("xcassets"), FileType::Resource);
        assert_eq!(FileType::from_extension("storyboard"), FileType::Resource);
        assert_eq!(FileType::from_extension("xib"), FileType::Resource);
        assert_eq!(FileType::from_extension("strings"), FileType::Resource);
        assert_eq!(FileType::from_extension("png"), FileType::Resource);
    }

    #[test]
    fn test_file_type_from_extension_framework() {
        assert_eq!(FileType::from_extension("framework"), FileType::Framework);
        assert_eq!(FileType::from_extension("xcframework"), FileType::Framework);
    }

    #[test]
    fn test_file_type_from_extension_unknown() {
        assert_eq!(FileType::from_extension("txt"), FileType::Unknown);
        assert_eq!(FileType::from_extension("xyz"), FileType::Unknown);
    }

    #[test]
    fn test_file_type_last_known_file_type() {
        assert_eq!(FileType::Swift.last_known_file_type(), "sourcecode.swift");
        assert_eq!(FileType::ObjC.last_known_file_type(), "sourcecode.c.objc");
        assert_eq!(FileType::Metal.last_known_file_type(), "sourcecode.metal");
        assert_eq!(FileType::Header.last_known_file_type(), "sourcecode.c.h");
        assert_eq!(FileType::Framework.last_known_file_type(), "wrapper.framework");
    }

    #[test]
    fn test_file_type_should_add_to_sources() {
        assert!(FileType::Swift.should_add_to_sources());
        assert!(FileType::ObjC.should_add_to_sources());
        assert!(FileType::Metal.should_add_to_sources());
        assert!(!FileType::Header.should_add_to_sources());
        assert!(!FileType::Resource.should_add_to_sources());
        assert!(!FileType::Framework.should_add_to_sources());
        assert!(!FileType::Unknown.should_add_to_sources());
    }

    #[test]
    fn test_quote_if_needed_simple() {
        assert_eq!(XcodeProject::quote_if_needed("simple"), "simple");
    }

    #[test]
    fn test_quote_if_needed_with_spaces() {
        assert_eq!(
            XcodeProject::quote_if_needed("path with spaces"),
            "\"path with spaces\""
        );
    }

    #[test]
    fn test_quote_if_needed_with_slashes() {
        assert_eq!(
            XcodeProject::quote_if_needed("FoodShare/Core/File.swift"),
            "\"FoodShare/Core/File.swift\""
        );
    }

    #[test]
    fn test_quote_if_needed_with_dots() {
        assert_eq!(
            XcodeProject::quote_if_needed("file.swift"),
            "\"file.swift\""
        );
    }

    #[test]
    fn test_quote_if_needed_with_dashes() {
        assert_eq!(
            XcodeProject::quote_if_needed("my-file"),
            "\"my-file\""
        );
    }
}
