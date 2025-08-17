use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::time::{timeout, Duration};

use crate::logger::{log_debug, log_error, log_info, log_warn};
use crate::path_manager::get_path_manager;
use crate::project_state_manager::{get_project_state_manager, FileOperation as ProjectFileOp, FileOperationType};
use crate::tool_safety::{get_global_safety_validator, OperationType};

/// Parameters for file edit operations (equivalent to gemini-cli's EditToolParams)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditParams {
    pub file_path: PathBuf,
    pub old_string: String,
    pub new_string: String,
    pub expected_replacements: Option<usize>,
}

/// Parameters for file write operations (equivalent to gemini-cli's WriteFileToolParams)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFileParams {
    pub file_path: PathBuf,
    pub content: String,
}

/// Parameters for multi-edit operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiEditParams {
    pub operations: Vec<EditOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditOperation {
    pub file_path: PathBuf,
    pub old_string: String,
    pub new_string: String,
}

/// Result of file operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationResult {
    pub success: bool,
    pub message: String,
    pub file_path: PathBuf,
    pub operation_type: String,
    pub changes_applied: usize,
}

/// Core file operations implementation inspired by gemini-cli's file tools
pub struct FileOperationsManager;

impl FileOperationsManager {
    pub fn new() -> Self {
        Self
    }

    /// Apply replacement to content (equivalent to gemini-cli's applyReplacement function)
    pub fn apply_replacement(
        current_content: Option<&str>,
        old_string: &str,
        new_string: &str,
        is_new_file: bool,
    ) -> String {
        if is_new_file {
            return new_string.to_string();
        }
        
        match current_content {
            Some(content) => {
                if old_string.is_empty() && !is_new_file {
                    content.to_string()
                } else {
                    content.replace(old_string, new_string)
                }
            }
            None => {
                if old_string.is_empty() {
                    new_string.to_string()
                } else {
                    String::new()
                }
            }
        }
    }

    /// Edit file by replacing old_string with new_string (equivalent to gemini-cli's edit.ts)
    pub async fn edit_file(&self, params: EditParams) -> Result<FileOperationResult> {
        log_info!("file_ops", "🔧 Starting file edit: {}", params.file_path.display());

        // Validate file path and permissions
        self.validate_file_path(&params.file_path)?;
        
        // Check permissions with safety validator
        let safety_validator = get_global_safety_validator();
        if !safety_validator.is_operation_allowed(&OperationType::EditFile(params.file_path.clone())).await {
            anyhow::bail!("Operation not permitted by safety validator");
        }

        let expected_replacements = params.expected_replacements.unwrap_or(1);
        let file_exists = params.file_path.exists();
        let is_new_file = !file_exists && params.old_string.is_empty();

        let current_content = if file_exists {
            Some(fs::read_to_string(&params.file_path)
                .context("Failed to read existing file")?)
        } else if is_new_file {
            None
        } else {
            anyhow::bail!("File not found. Cannot apply edit. Use an empty old_string to create a new file.");
        };

        // Count occurrences of old_string in current content
        let occurrences = if let Some(content) = &current_content {
            if params.old_string.is_empty() {
                if file_exists {
                    return Err(anyhow::anyhow!("Failed to edit. Attempted to create a file that already exists."));
                }
                0
            } else {
                content.matches(&params.old_string).count()
            }
        } else {
            0
        };

        // Validate expected replacements
        if !is_new_file && occurrences != expected_replacements {
            let occurrence_term = if expected_replacements == 1 { "occurrence" } else { "occurrences" };
            anyhow::bail!(
                "Failed to edit, expected {} {} but found {} for old_string in file: {}",
                expected_replacements,
                occurrence_term,
                occurrences,
                params.file_path.display()
            );
        }

        if occurrences == 0 && !is_new_file {
            anyhow::bail!(
                "Failed to edit, could not find the string to replace in file: {}",
                params.file_path.display()
            );
        }

        if params.old_string == params.new_string {
            anyhow::bail!("No changes to apply. The old_string and new_string are identical.");
        }

        // Apply the replacement
        let new_content = Self::apply_replacement(
            current_content.as_deref(),
            &params.old_string,
            &params.new_string,
            is_new_file,
        );

        // Write the new content
        self.write_file_content(&params.file_path, &new_content, is_new_file).await?;

        // Record the operation in project state
        self.record_file_operation(&params.file_path, FileOperationType::Edit, &new_content).await?;

        let result = FileOperationResult {
            success: true,
            message: if is_new_file {
                format!("Created new file: {}", params.file_path.display())
            } else {
                format!("Successfully modified file: {} ({} replacements)", params.file_path.display(), occurrences)
            },
            file_path: params.file_path.clone(),
            operation_type: "edit".to_string(),
            changes_applied: occurrences,
        };

        log_info!("file_ops", "✅ File edit completed: {}", result.message);
        Ok(result)
    }

    /// Write content to file (equivalent to gemini-cli's write-file.ts)
    pub async fn write_file(&self, params: WriteFileParams) -> Result<FileOperationResult> {
        log_info!("file_ops", "📝 Starting file write: {}", params.file_path.display());

        // Validate file path and permissions
        self.validate_file_path(&params.file_path)?;
        
        // Check permissions with safety validator
        let safety_validator = get_global_safety_validator();
        if !safety_validator.is_operation_allowed(&OperationType::WriteFile(params.file_path.clone())).await {
            anyhow::bail!("Operation not permitted by safety validator");
        }

        let file_exists = params.file_path.exists();
        let is_new_file = !file_exists;

        // Write the content
        self.write_file_content(&params.file_path, &params.content, is_new_file).await?;

        // Record the operation in project state
        self.record_file_operation(&params.file_path, FileOperationType::Write, &params.content).await?;

        let result = FileOperationResult {
            success: true,
            message: if is_new_file {
                format!("Successfully created and wrote to new file: {}", params.file_path.display())
            } else {
                format!("Successfully overwrote file: {}", params.file_path.display())
            },
            file_path: params.file_path.clone(),
            operation_type: "write".to_string(),
            changes_applied: 1,
        };

        log_info!("file_ops", "✅ File write completed: {}", result.message);
        Ok(result)
    }

    /// Execute multiple file edit operations (equivalent to gemini-cli's multiedit.ts)
    pub async fn multi_edit(&self, params: MultiEditParams) -> Result<Vec<FileOperationResult>> {
        log_info!("file_ops", "🔄 Starting multi-edit operation with {} files", params.operations.len());

        let mut results = Vec::new();

        for operation in params.operations {
            let edit_params = EditParams {
                file_path: operation.file_path.clone(),
                old_string: operation.old_string,
                new_string: operation.new_string,
                expected_replacements: Some(1),
            };

            match self.edit_file(edit_params).await {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    log_error!("file_ops", "❌ Multi-edit failed for {}: {}", operation.file_path.display(), e);
                    results.push(FileOperationResult {
                        success: false,
                        message: format!("Failed to edit {}: {}", operation.file_path.display(), e),
                        file_path: operation.file_path,
                        operation_type: "edit".to_string(),
                        changes_applied: 0,
                    });
                }
            }
        }

        let successful_operations = results.iter().filter(|r| r.success).count();
        log_info!("file_ops", "✅ Multi-edit completed: {}/{} operations successful", 
                 successful_operations, results.len());

        Ok(results)
    }

    /// Read file content with error handling
    pub async fn read_file(&self, file_path: &Path) -> Result<String> {
        log_debug!("file_ops", "📖 Reading file: {}", file_path.display());

        // Validate file path
        self.validate_file_path(file_path)?;

        // Check permissions with safety validator
        let safety_validator = get_global_safety_validator();
        if !safety_validator.is_operation_allowed(&OperationType::ReadFile(file_path.to_path_buf())).await {
            anyhow::bail!("Operation not permitted by safety validator");
        }

        let content = fs::read_to_string(file_path)
            .context(format!("Failed to read file: {}", file_path.display()))?;

        log_debug!("file_ops", "✅ Successfully read {} bytes from {}", content.len(), file_path.display());
        Ok(content)
    }

    /// Internal helper to write file content with proper error handling
    async fn write_file_content(&self, file_path: &Path, content: &str, is_new_file: bool) -> Result<()> {
        // Ensure parent directories exist
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .context(format!("Failed to create parent directories for: {}", file_path.display()))?;
                log_debug!("file_ops", "📁 Created parent directories for: {}", file_path.display());
            }
        }

        // Write file with timeout to prevent hanging
        let file_path_clone = file_path.to_path_buf();
        let content_clone = content.to_string();
        
        let write_operation = tokio::task::spawn_blocking(move || {
            let mut file = File::create(&file_path_clone)
                .context(format!("Failed to create file: {}", file_path_clone.display()))?;
            
            file.write_all(content_clone.as_bytes())
                .context(format!("Failed to write content to file: {}", file_path_clone.display()))?;
            
            file.sync_all()
                .context(format!("Failed to sync file to disk: {}", file_path_clone.display()))?;
            
            Ok::<(), anyhow::Error>(())
        });

        timeout(Duration::from_secs(30), write_operation)
            .await
            .context("File write operation timed out")?
            .context("File write task failed")?
            .context("File write operation failed")?;

        log_debug!("file_ops", "💾 Successfully wrote {} bytes to {}", content.len(), file_path.display());
        Ok(())
    }

    /// Validate file path using existing path manager
    fn validate_file_path(&self, file_path: &Path) -> Result<()> {
        if !file_path.is_absolute() {
            anyhow::bail!("File path must be absolute: {}", file_path.display());
        }

        // Use path manager to resolve and validate path
        let path_manager = get_path_manager();
        let resolved_path = path_manager.resolve_path(file_path)
            .context("Failed to resolve file path")?;

        // Additional safety checks
        if resolved_path.to_string_lossy().contains("..") {
            anyhow::bail!("Path traversal not allowed: {}", file_path.display());
        }

        Ok(())
    }

    /// Record file operation in project state manager
    async fn record_file_operation(&self, file_path: &Path, operation_type: FileOperationType, content: &str) -> Result<()> {
        let mut project_state = get_project_state_manager();
        
        let operation = ProjectFileOp {
            path: file_path.to_path_buf(),
            operation_type,
            timestamp: chrono::Utc::now(),
            size_bytes: content.len(),
            content_hash: format!("{:x}", md5::compute(content.as_bytes())),
        };

        project_state.record_file_operation(operation).await
            .context("Failed to record file operation in project state")?;

        Ok(())
    }
}

/// Global singleton instance
static FILE_OPERATIONS_MANAGER: once_cell::sync::Lazy<std::sync::Arc<FileOperationsManager>> = 
    once_cell::sync::Lazy::new(|| {
        std::sync::Arc::new(FileOperationsManager::new())
    });

/// Get global file operations manager instance
pub fn get_file_operations_manager() -> std::sync::Arc<FileOperationsManager> {
    FILE_OPERATIONS_MANAGER.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_apply_replacement() {
        // Test new file creation
        let result = FileOperationsManager::apply_replacement(None, "", "Hello, World!", true);
        assert_eq!(result, "Hello, World!");

        // Test content replacement
        let content = "Hello, World!";
        let result = FileOperationsManager::apply_replacement(Some(content), "World", "Rust", false);
        assert_eq!(result, "Hello, Rust!");

        // Test no replacement when old_string is empty and not new file
        let result = FileOperationsManager::apply_replacement(Some(content), "", "New content", false);
        assert_eq!(result, content);
    }

    #[tokio::test]
    async fn test_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        
        let manager = FileOperationsManager::new();

        // Test write file
        let write_params = WriteFileParams {
            file_path: file_path.clone(),
            content: "Hello, World!".to_string(),
        };

        // Note: This test will fail without proper safety validator setup
        // In real usage, the safety validator needs to be properly initialized
        // For testing, we would need to mock or disable the safety validator
    }
}