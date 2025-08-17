use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::logger::{log_debug, log_info, log_warn};
use crate::file_operations::{get_file_operations_manager, WriteFileParams};

/// Package.json dependency management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageJson {
    pub name: String,
    pub version: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub package_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scripts: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies", skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "peerDependencies", skip_serializing_if = "Option::is_none")]
    pub peer_dependencies: Option<HashMap<String, String>>,
    #[serde(flatten)]
    pub additional_fields: HashMap<String, serde_json::Value>,
}

/// Dependency type for installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    /// Runtime dependency
    Production,
    /// Development dependency
    Development,
    /// Peer dependency
    Peer,
}

/// Dependency operation for batch processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyOperation {
    pub name: String,
    pub version: Option<String>,
    pub dependency_type: DependencyType,
    pub operation: DependencyOpType,
}

/// Type of dependency operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyOpType {
    /// Add new dependency
    Add,
    /// Remove existing dependency
    Remove,
    /// Update existing dependency
    Update,
}

/// Result of dependency operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResult {
    /// Project root directory
    pub project_root: PathBuf,
    /// Operations that were performed
    pub operations: Vec<DependencyOperation>,
    /// Whether package.json was modified
    pub package_json_modified: bool,
    /// New dependencies that were added
    pub added_dependencies: Vec<String>,
    /// Dependencies that were removed
    pub removed_dependencies: Vec<String>,
    /// Dependencies that were updated
    pub updated_dependencies: Vec<String>,
    /// Execution time
    pub execution_time: f64,
}

/// Package.json and dependency management system
pub struct DependencyManager {
    file_ops: std::sync::Arc<crate::file_operations::FileOperationsManager>,
}

impl DependencyManager {
    pub fn new() -> Self {
        Self {
            file_ops: get_file_operations_manager(),
        }
    }

    /// Read and parse package.json from a project directory
    pub async fn read_package_json(&self, project_root: &PathBuf) -> Result<PackageJson> {
        log_debug!("dep_mgr", "📖 Reading package.json from {}", project_root.display());
        
        let package_json_path = project_root.join("package.json");
        let content = self.file_ops.read_file(&package_json_path).await
            .context("Failed to read package.json file")?;

        let package_json: PackageJson = serde_json::from_str(&content)
            .context("Failed to parse package.json as valid JSON")?;

        log_info!("dep_mgr", "✅ Successfully parsed package.json for project: {}", package_json.name);
        Ok(package_json)
    }

    /// Write package.json to a project directory
    pub async fn write_package_json(&self, project_root: &PathBuf, package_json: &PackageJson) -> Result<()> {
        log_debug!("dep_mgr", "💾 Writing package.json to {}", project_root.display());
        
        let package_json_path = project_root.join("package.json");
        let content = serde_json::to_string_pretty(package_json)
            .context("Failed to serialize package.json to JSON")?;

        let write_params = WriteFileParams {
            file_path: package_json_path,
            content,
        };

        self.file_ops.write_file(write_params).await?;
        log_info!("dep_mgr", "✅ Successfully wrote package.json for project: {}", package_json.name);
        Ok(())
    }

    /// Add a single dependency to package.json
    pub async fn add_dependency(&self, 
        project_root: &PathBuf, 
        name: &str, 
        version: &str, 
        dependency_type: DependencyType
    ) -> Result<DependencyResult> {
        let start_time = std::time::Instant::now();
        log_info!("dep_mgr", "➕ Adding {} dependency: {}@{}", 
                 self.dependency_type_name(&dependency_type), name, version);

        let mut package_json = self.read_package_json(project_root).await?;
        let mut modified = false;

        match dependency_type {
            DependencyType::Production => {
                if package_json.dependencies.is_none() {
                    package_json.dependencies = Some(HashMap::new());
                }
                let deps = package_json.dependencies.as_mut().unwrap();
                if !deps.contains_key(name) || deps[name] != version {
                    deps.insert(name.to_string(), version.to_string());
                    modified = true;
                }
            }
            DependencyType::Development => {
                if package_json.dev_dependencies.is_none() {
                    package_json.dev_dependencies = Some(HashMap::new());
                }
                let deps = package_json.dev_dependencies.as_mut().unwrap();
                if !deps.contains_key(name) || deps[name] != version {
                    deps.insert(name.to_string(), version.to_string());
                    modified = true;
                }
            }
            DependencyType::Peer => {
                if package_json.peer_dependencies.is_none() {
                    package_json.peer_dependencies = Some(HashMap::new());
                }
                let deps = package_json.peer_dependencies.as_mut().unwrap();
                if !deps.contains_key(name) || deps[name] != version {
                    deps.insert(name.to_string(), version.to_string());
                    modified = true;
                }
            }
        }

        if modified {
            self.write_package_json(project_root, &package_json).await?;
        }

        let execution_time = start_time.elapsed().as_secs_f64();
        let operation = DependencyOperation {
            name: name.to_string(),
            version: Some(version.to_string()),
            dependency_type,
            operation: DependencyOpType::Add,
        };

        Ok(DependencyResult {
            project_root: project_root.clone(),
            operations: vec![operation],
            package_json_modified: modified,
            added_dependencies: if modified { vec![name.to_string()] } else { vec![] },
            removed_dependencies: vec![],
            updated_dependencies: vec![],
            execution_time,
        })
    }

    /// Remove a dependency from package.json
    pub async fn remove_dependency(&self, 
        project_root: &PathBuf, 
        name: &str
    ) -> Result<DependencyResult> {
        let start_time = std::time::Instant::now();
        log_info!("dep_mgr", "➖ Removing dependency: {}", name);

        let mut package_json = self.read_package_json(project_root).await?;
        let mut modified = false;
        let mut removed_from = DependencyType::Production; // Track which section it was removed from

        // Check and remove from all dependency sections
        if let Some(deps) = package_json.dependencies.as_mut() {
            if deps.remove(name).is_some() {
                modified = true;
                removed_from = DependencyType::Production;
            }
        }
        if let Some(deps) = package_json.dev_dependencies.as_mut() {
            if deps.remove(name).is_some() {
                modified = true;
                removed_from = DependencyType::Development;
            }
        }
        if let Some(deps) = package_json.peer_dependencies.as_mut() {
            if deps.remove(name).is_some() {
                modified = true;
                removed_from = DependencyType::Peer;
            }
        }

        if modified {
            self.write_package_json(project_root, &package_json).await?;
        }

        let execution_time = start_time.elapsed().as_secs_f64();
        let operation = DependencyOperation {
            name: name.to_string(),
            version: None,
            dependency_type: removed_from,
            operation: DependencyOpType::Remove,
        };

        Ok(DependencyResult {
            project_root: project_root.clone(),
            operations: vec![operation],
            package_json_modified: modified,
            added_dependencies: vec![],
            removed_dependencies: if modified { vec![name.to_string()] } else { vec![] },
            updated_dependencies: vec![],
            execution_time,
        })
    }

    /// Update a dependency version in package.json
    pub async fn update_dependency(&self, 
        project_root: &PathBuf, 
        name: &str, 
        version: &str
    ) -> Result<DependencyResult> {
        let start_time = std::time::Instant::now();
        log_info!("dep_mgr", "🔄 Updating dependency: {}@{}", name, version);

        let mut package_json = self.read_package_json(project_root).await?;
        let mut modified = false;
        let mut updated_in = DependencyType::Production; // Track which section it was updated in

        // Check and update in all dependency sections
        if let Some(deps) = package_json.dependencies.as_mut() {
            if let Some(current_version) = deps.get_mut(name) {
                if *current_version != version {
                    *current_version = version.to_string();
                    modified = true;
                    updated_in = DependencyType::Production;
                }
            }
        }
        if let Some(deps) = package_json.dev_dependencies.as_mut() {
            if let Some(current_version) = deps.get_mut(name) {
                if *current_version != version {
                    *current_version = version.to_string();
                    modified = true;
                    updated_in = DependencyType::Development;
                }
            }
        }
        if let Some(deps) = package_json.peer_dependencies.as_mut() {
            if let Some(current_version) = deps.get_mut(name) {
                if *current_version != version {
                    *current_version = version.to_string();
                    modified = true;
                    updated_in = DependencyType::Peer;
                }
            }
        }

        if modified {
            self.write_package_json(project_root, &package_json).await?;
        }

        let execution_time = start_time.elapsed().as_secs_f64();
        let operation = DependencyOperation {
            name: name.to_string(),
            version: Some(version.to_string()),
            dependency_type: updated_in,
            operation: DependencyOpType::Update,
        };

        Ok(DependencyResult {
            project_root: project_root.clone(),
            operations: vec![operation],
            package_json_modified: modified,
            added_dependencies: vec![],
            removed_dependencies: vec![],
            updated_dependencies: if modified { vec![name.to_string()] } else { vec![] },
            execution_time,
        })
    }

    /// Perform batch dependency operations
    pub async fn batch_operations(&self, 
        project_root: &PathBuf, 
        operations: Vec<DependencyOperation>
    ) -> Result<DependencyResult> {
        let start_time = std::time::Instant::now();
        log_info!("dep_mgr", "🔄 Performing {} dependency operations", operations.len());

        let mut package_json = self.read_package_json(project_root).await?;
        let mut modified = false;
        let mut added_dependencies = Vec::new();
        let mut removed_dependencies = Vec::new();
        let mut updated_dependencies = Vec::new();

        for op in &operations {
            let deps_map = match op.dependency_type {
                DependencyType::Production => {
                    if package_json.dependencies.is_none() {
                        package_json.dependencies = Some(HashMap::new());
                    }
                    package_json.dependencies.as_mut().unwrap()
                }
                DependencyType::Development => {
                    if package_json.dev_dependencies.is_none() {
                        package_json.dev_dependencies = Some(HashMap::new());
                    }
                    package_json.dev_dependencies.as_mut().unwrap()
                }
                DependencyType::Peer => {
                    if package_json.peer_dependencies.is_none() {
                        package_json.peer_dependencies = Some(HashMap::new());
                    }
                    package_json.peer_dependencies.as_mut().unwrap()
                }
            };

            match op.operation {
                DependencyOpType::Add => {
                    if let Some(version) = &op.version {
                        if !deps_map.contains_key(&op.name) || deps_map[&op.name] != *version {
                            deps_map.insert(op.name.clone(), version.clone());
                            added_dependencies.push(op.name.clone());
                            modified = true;
                        }
                    }
                }
                DependencyOpType::Remove => {
                    if deps_map.remove(&op.name).is_some() {
                        removed_dependencies.push(op.name.clone());
                        modified = true;
                    }
                }
                DependencyOpType::Update => {
                    if let Some(version) = &op.version {
                        if let Some(current_version) = deps_map.get_mut(&op.name) {
                            if *current_version != *version {
                                *current_version = version.clone();
                                updated_dependencies.push(op.name.clone());
                                modified = true;
                            }
                        }
                    }
                }
            }
        }

        if modified {
            self.write_package_json(project_root, &package_json).await?;
        }

        let execution_time = start_time.elapsed().as_secs_f64();
        log_info!("dep_mgr", "✅ Batch operations completed: {} added, {} removed, {} updated", 
                 added_dependencies.len(), removed_dependencies.len(), updated_dependencies.len());

        Ok(DependencyResult {
            project_root: project_root.clone(),
            operations,
            package_json_modified: modified,
            added_dependencies,
            removed_dependencies,
            updated_dependencies,
            execution_time,
        })
    }

    /// Add or update a script in package.json
    pub async fn add_script(&self, 
        project_root: &PathBuf, 
        script_name: &str, 
        script_command: &str
    ) -> Result<()> {
        log_info!("dep_mgr", "📝 Adding script '{}': {}", script_name, script_command);

        let mut package_json = self.read_package_json(project_root).await?;
        
        if package_json.scripts.is_none() {
            package_json.scripts = Some(HashMap::new());
        }
        
        let scripts = package_json.scripts.as_mut().unwrap();
        scripts.insert(script_name.to_string(), script_command.to_string());

        self.write_package_json(project_root, &package_json).await?;
        log_info!("dep_mgr", "✅ Script '{}' added successfully", script_name);
        Ok(())
    }

    /// Remove a script from package.json
    pub async fn remove_script(&self, 
        project_root: &PathBuf, 
        script_name: &str
    ) -> Result<()> {
        log_info!("dep_mgr", "🗑️ Removing script '{}'", script_name);

        let mut package_json = self.read_package_json(project_root).await?;
        
        if let Some(scripts) = package_json.scripts.as_mut() {
            if scripts.remove(script_name).is_some() {
                self.write_package_json(project_root, &package_json).await?;
                log_info!("dep_mgr", "✅ Script '{}' removed successfully", script_name);
            } else {
                log_warn!("dep_mgr", "⚠️ Script '{}' not found", script_name);
            }
        } else {
            log_warn!("dep_mgr", "⚠️ No scripts section found in package.json");
        }
        Ok(())
    }

    /// List all dependencies with their versions
    pub async fn list_dependencies(&self, project_root: &PathBuf) -> Result<HashMap<String, (String, DependencyType)>> {
        log_debug!("dep_mgr", "📋 Listing all dependencies");

        let package_json = self.read_package_json(project_root).await?;
        let mut all_deps = HashMap::new();

        if let Some(deps) = &package_json.dependencies {
            for (name, version) in deps {
                all_deps.insert(name.clone(), (version.clone(), DependencyType::Production));
            }
        }

        if let Some(deps) = &package_json.dev_dependencies {
            for (name, version) in deps {
                all_deps.insert(name.clone(), (version.clone(), DependencyType::Development));
            }
        }

        if let Some(deps) = &package_json.peer_dependencies {
            for (name, version) in deps {
                all_deps.insert(name.clone(), (version.clone(), DependencyType::Peer));
            }
        }

        log_info!("dep_mgr", "📋 Found {} total dependencies", all_deps.len());
        Ok(all_deps)
    }

    /// Check if a dependency exists in package.json
    pub async fn has_dependency(&self, project_root: &PathBuf, name: &str) -> Result<Option<(String, DependencyType)>> {
        let package_json = self.read_package_json(project_root).await?;

        if let Some(deps) = &package_json.dependencies {
            if let Some(version) = deps.get(name) {
                return Ok(Some((version.clone(), DependencyType::Production)));
            }
        }

        if let Some(deps) = &package_json.dev_dependencies {
            if let Some(version) = deps.get(name) {
                return Ok(Some((version.clone(), DependencyType::Development)));
            }
        }

        if let Some(deps) = &package_json.peer_dependencies {
            if let Some(version) = deps.get(name) {
                return Ok(Some((version.clone(), DependencyType::Peer)));
            }
        }

        Ok(None)
    }

    /// Helper function to get dependency type name
    fn dependency_type_name(&self, dep_type: &DependencyType) -> &'static str {
        match dep_type {
            DependencyType::Production => "production",
            DependencyType::Development => "development",
            DependencyType::Peer => "peer",
        }
    }

    /// Add multiple dependencies from a list of package specifications
    pub async fn add_dependencies_from_specs(&self, 
        project_root: &PathBuf, 
        specs: &[(&str, &str, DependencyType)]
    ) -> Result<DependencyResult> {
        let operations: Vec<DependencyOperation> = specs.iter()
            .map(|(name, version, dep_type)| DependencyOperation {
                name: name.to_string(),
                version: Some(version.to_string()),
                dependency_type: dep_type.clone(),
                operation: DependencyOpType::Add,
            })
            .collect();

        self.batch_operations(project_root, operations).await
    }

    /// Update package.json metadata (name, version, description, etc.)
    pub async fn update_metadata(&self, 
        project_root: &PathBuf, 
        name: Option<&str>,
        version: Option<&str>,
        description: Option<&str>,
        author: Option<&str>,
        license: Option<&str>
    ) -> Result<()> {
        log_info!("dep_mgr", "📝 Updating package.json metadata");

        let mut package_json = self.read_package_json(project_root).await?;
        let mut modified = false;

        if let Some(name) = name {
            if package_json.name != name {
                package_json.name = name.to_string();
                modified = true;
            }
        }

        if let Some(version) = version {
            if package_json.version != version {
                package_json.version = version.to_string();
                modified = true;
            }
        }

        if let Some(description) = description {
            if package_json.description.as_deref() != Some(description) {
                package_json.description = Some(description.to_string());
                modified = true;
            }
        }

        if let Some(author) = author {
            if package_json.author.as_deref() != Some(author) {
                package_json.author = Some(author.to_string());
                modified = true;
            }
        }

        if let Some(license) = license {
            if package_json.license.as_deref() != Some(license) {
                package_json.license = Some(license.to_string());
                modified = true;
            }
        }

        if modified {
            self.write_package_json(project_root, &package_json).await?;
            log_info!("dep_mgr", "✅ Package.json metadata updated");
        } else {
            log_debug!("dep_mgr", "📋 No metadata changes needed");
        }

        Ok(())
    }
}

/// Global singleton instance
static DEPENDENCY_MANAGER: once_cell::sync::Lazy<std::sync::Arc<DependencyManager>> = 
    once_cell::sync::Lazy::new(|| {
        std::sync::Arc::new(DependencyManager::new())
    });

/// Get global dependency manager instance
pub fn get_dependency_manager() -> std::sync::Arc<DependencyManager> {
    DEPENDENCY_MANAGER.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_dependency_operations() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let dep_manager = DependencyManager::new();

        // Create initial package.json
        let initial_package = PackageJson {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            package_type: Some("module".to_string()),
            description: Some("Test package".to_string()),
            author: None,
            license: Some("MIT".to_string()),
            scripts: None,
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            additional_fields: HashMap::new(),
        };

        dep_manager.write_package_json(&project_root, &initial_package).await.unwrap();

        // Test adding a dependency
        let result = dep_manager.add_dependency(
            &project_root, 
            "react", 
            "^18.0.0", 
            DependencyType::Production
        ).await.unwrap();

        assert!(result.package_json_modified);
        assert_eq!(result.added_dependencies.len(), 1);
        assert_eq!(result.added_dependencies[0], "react");

        // Test checking dependency exists
        let dep_info = dep_manager.has_dependency(&project_root, "react").await.unwrap();
        assert!(dep_info.is_some());
        assert_eq!(dep_info.unwrap().0, "^18.0.0");

        // Test removing dependency
        let result = dep_manager.remove_dependency(&project_root, "react").await.unwrap();
        assert!(result.package_json_modified);
        assert_eq!(result.removed_dependencies.len(), 1);
        assert_eq!(result.removed_dependencies[0], "react");
    }

    #[tokio::test]
    async fn test_script_management() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let dep_manager = DependencyManager::new();

        // Create initial package.json
        let initial_package = PackageJson {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            package_type: None,
            description: None,
            author: None,
            license: None,
            scripts: None,
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            additional_fields: HashMap::new(),
        };

        dep_manager.write_package_json(&project_root, &initial_package).await.unwrap();

        // Test adding script
        dep_manager.add_script(&project_root, "test", "jest").await.unwrap();

        // Verify script was added
        let package_json = dep_manager.read_package_json(&project_root).await.unwrap();
        assert!(package_json.scripts.is_some());
        assert_eq!(package_json.scripts.unwrap()["test"], "jest");

        // Test removing script
        dep_manager.remove_script(&project_root, "test").await.unwrap();

        // Verify script was removed
        let package_json = dep_manager.read_package_json(&project_root).await.unwrap();
        assert!(package_json.scripts.is_none() || !package_json.scripts.unwrap().contains_key("test"));
    }
}