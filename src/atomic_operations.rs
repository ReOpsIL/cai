use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

use crate::file_safety::{FileSafetyManager, SafeWriteOperation};
use crate::permission_manager::{PermissionManager, PermissionRequest, PermissionAction, RiskLevel, SessionId};

/// Atomic multi-file operation manager ensuring all-or-nothing semantics
#[derive(Debug)]
pub struct AtomicMultiFileOperation {
    operation_id: String,
    operations: Vec<FileOperation>,
    completed_operations: Vec<SafeWriteOperation>,
    rollback_data: Vec<RollbackData>,
    temp_dir: PathBuf,
    session_id: SessionId,
}

/// Individual file operation within an atomic transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOperation {
    Create { 
        path: PathBuf, 
        content: String,
        description: String,
    },
    Modify { 
        path: PathBuf, 
        old_content: String, 
        new_content: String,
        description: String,
    },
    Delete { 
        path: PathBuf,
        description: String,
    },
    Move { 
        from_path: PathBuf, 
        to_path: PathBuf,
        description: String,
    },
    Copy { 
        from_path: PathBuf, 
        to_path: PathBuf,
        description: String,
    },
}

/// Rollback information for each operation
#[derive(Debug, Clone)]
pub struct RollbackData {
    operation_id: String,
    operation_type: OperationType,
    target_path: PathBuf,
    backup_path: Option<PathBuf>,
    original_existed: bool,
}

#[derive(Debug, Clone)]
pub enum OperationType {
    Create,
    Modify,
    Delete,
    Move,
    Copy,
}

/// Result of an atomic multi-file operation
#[derive(Debug, Clone)]
pub struct AtomicOperationResult {
    pub operation_id: String,
    pub success: bool,
    pub operations_completed: usize,
    pub operations_total: usize,
    pub errors: Vec<String>,
    pub files_affected: Vec<PathBuf>,
}

/// Validation result for multi-file operations
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub permission_requests: Vec<PermissionRequest>,
}

impl AtomicMultiFileOperation {
    /// Create a new atomic multi-file operation
    pub async fn new(
        operations: Vec<FileOperation>,
        session_id: SessionId,
    ) -> Result<Self> {
        let operation_id = Uuid::new_v4().to_string();
        let temp_dir = std::env::temp_dir().join(format!("cai-atomic-{}", operation_id));
        fs::create_dir_all(&temp_dir).await?;

        Ok(Self {
            operation_id,
            operations,
            completed_operations: Vec::new(),
            rollback_data: Vec::new(),
            temp_dir,
            session_id,
        })
    }

    /// Validate all operations before execution
    pub async fn validate(
        &self,
        permission_manager: &PermissionManager,
        file_safety: &FileSafetyManager,
    ) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut permission_requests = Vec::new();

        for (index, operation) in self.operations.iter().enumerate() {
            // Validate individual operation
            match self.validate_operation(operation, file_safety).await {
                Ok(op_warnings) => warnings.extend(op_warnings),
                Err(e) => errors.push(format!("Operation {}: {}", index + 1, e)),
            }

            // Check permissions
            let perm_requests = self.check_operation_permissions(operation, permission_manager)?;
            permission_requests.extend(perm_requests);
        }

        // Check for conflicts between operations
        if let Err(conflict_errors) = self.check_operation_conflicts() {
            errors.extend(conflict_errors.iter().map(|e| e.to_string()));
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
            permission_requests,
        })
    }

    /// Execute all operations atomically
    pub async fn execute(
        mut self,
        file_safety: &mut FileSafetyManager,
        permission_manager: &PermissionManager,
    ) -> Result<AtomicOperationResult> {
        // Phase 1: Validate all operations
        let validation = self.validate(permission_manager, file_safety).await?;
        if !validation.valid {
            return Ok(AtomicOperationResult {
                operation_id: self.operation_id.clone(),
                success: false,
                operations_completed: 0,
                operations_total: self.operations.len(),
                errors: validation.errors,
                files_affected: Vec::new(),
            });
        }

        // Phase 2: Create rollback data for all operations
        if let Err(e) = self.create_rollback_data().await {
            return Ok(AtomicOperationResult {
                operation_id: self.operation_id.clone(),
                success: false,
                operations_completed: 0,
                operations_total: self.operations.len(),
                errors: vec![format!("Failed to create rollback data: {}", e)],
                files_affected: Vec::new(),
            });
        }

        // Phase 3: Execute all operations
        let mut completed_count = 0;
        let mut affected_files = Vec::new();
        let mut execution_errors = Vec::new();

        for operation in &self.operations {
            match self.execute_single_operation(operation, file_safety).await {
                Ok(safe_op) => {
                    self.completed_operations.push(safe_op);
                    completed_count += 1;
                    affected_files.extend(self.get_operation_paths(operation));
                }
                Err(e) => {
                    execution_errors.push(format!("Failed to execute operation: {}", e));
                    // Rollback all completed operations
                    if let Err(rollback_err) = self.rollback(file_safety).await {
                        execution_errors.push(format!("Rollback failed: {}", rollback_err));
                    }
                    break;
                }
            }
        }

        let success = execution_errors.is_empty() && completed_count == self.operations.len();

        // Clean up temporary files if successful
        if success {
            let _ = self.cleanup().await;
        }

        Ok(AtomicOperationResult {
            operation_id: self.operation_id,
            success,
            operations_completed: completed_count,
            operations_total: self.operations.len(),
            errors: execution_errors,
            files_affected: affected_files,
        })
    }

    /// Validate a single operation
    async fn validate_operation(
        &self,
        operation: &FileOperation,
        file_safety: &FileSafetyManager,
    ) -> Result<Vec<String>, anyhow::Error> {
        let mut warnings = Vec::new();

        match operation {
            FileOperation::Create { path, .. } => {
                if path.exists() {
                    return Err(anyhow!("File already exists: {}", path.display()));
                }
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        warnings.push(format!("Parent directory will be created: {}", parent.display()));
                    }
                }
            }
            FileOperation::Modify { path, old_content, .. } => {
                if !path.exists() {
                    return Err(anyhow!("File does not exist: {}", path.display()));
                }
                if !file_safety.is_safe_to_read(path).await? {
                    return Err(anyhow!("File is not safe to read: {}", path.display()));
                }
                // Validate that old_content matches current content
                let current_content = fs::read_to_string(path).await?;
                if current_content != *old_content {
                    return Err(anyhow!("File content has changed since last read: {}", path.display()));
                }
            }
            FileOperation::Delete { path, .. } => {
                if !path.exists() {
                    warnings.push(format!("File to delete does not exist: {}", path.display()));
                }
            }
            FileOperation::Move { from_path, to_path, .. } => {
                if !from_path.exists() {
                    return Err(anyhow!("Source file does not exist: {}", from_path.display()));
                }
                if to_path.exists() {
                    return Err(anyhow!("Destination file already exists: {}", to_path.display()));
                }
            }
            FileOperation::Copy { from_path, to_path, .. } => {
                if !from_path.exists() {
                    return Err(anyhow!("Source file does not exist: {}", from_path.display()));
                }
                if to_path.exists() {
                    warnings.push(format!("Destination file will be overwritten: {}", to_path.display()));
                }
            }
        }

        Ok(warnings)
    }

    /// Check permissions for an operation
    fn check_operation_permissions(
        &self,
        operation: &FileOperation,
        permission_manager: &PermissionManager,
    ) -> Result<Vec<PermissionRequest>> {
        let mut requests = Vec::new();

        match operation {
            FileOperation::Create { path, description, .. } => {
                if !permission_manager.check_permission(
                    &self.session_id,
                    "file_operations",
                    &PermissionAction::Create,
                    Some(path),
                )? {
                    requests.push(PermissionRequest {
                        id: Uuid::new_v4().to_string(),
                        session_id: self.session_id.clone(),
                        tool_call_id: Some(self.operation_id.clone()),
                        tool_name: "file_operations".to_string(),
                        action: PermissionAction::Create,
                        path: Some(path.clone()),
                        description: description.clone(),
                        justification: "Required for atomic file operation".to_string(),
                        risk_level: RiskLevel::Medium,
                    });
                }
            }
            FileOperation::Modify { path, description, .. } => {
                if !permission_manager.check_permission(
                    &self.session_id,
                    "file_operations",
                    &PermissionAction::Write,
                    Some(path),
                )? {
                    requests.push(PermissionRequest {
                        id: Uuid::new_v4().to_string(),
                        session_id: self.session_id.clone(),
                        tool_call_id: Some(self.operation_id.clone()),
                        tool_name: "file_operations".to_string(),
                        action: PermissionAction::Write,
                        path: Some(path.clone()),
                        description: description.clone(),
                        justification: "Required for atomic file operation".to_string(),
                        risk_level: RiskLevel::Medium,
                    });
                }
            }
            FileOperation::Delete { path, description, .. } => {
                if !permission_manager.check_permission(
                    &self.session_id,
                    "file_operations",
                    &PermissionAction::Delete,
                    Some(path),
                )? {
                    requests.push(PermissionRequest {
                        id: Uuid::new_v4().to_string(),
                        session_id: self.session_id.clone(),
                        tool_call_id: Some(self.operation_id.clone()),
                        tool_name: "file_operations".to_string(),
                        action: PermissionAction::Delete,
                        path: Some(path.clone()),
                        description: description.clone(),
                        justification: "Required for atomic file operation".to_string(),
                        risk_level: RiskLevel::High,
                    });
                }
            }
            FileOperation::Move { from_path, to_path, description, .. } => {
                // Need both read and delete on source, create on destination
                for (path, action, risk) in [
                    (from_path, PermissionAction::Read, RiskLevel::Low),
                    (from_path, PermissionAction::Delete, RiskLevel::High),
                    (to_path, PermissionAction::Create, RiskLevel::Medium),
                ] {
                    if !permission_manager.check_permission(
                        &self.session_id,
                        "file_operations",
                        &action,
                        Some(path),
                    )? {
                        requests.push(PermissionRequest {
                            id: Uuid::new_v4().to_string(),
                            session_id: self.session_id.clone(),
                            tool_call_id: Some(self.operation_id.clone()),
                            tool_name: "file_operations".to_string(),
                            action,
                            path: Some(path.clone()),
                            description: description.clone(),
                            justification: "Required for atomic file operation".to_string(),
                            risk_level: risk,
                        });
                    }
                }
            }
            FileOperation::Copy { from_path, to_path, description, .. } => {
                // Need read on source, create on destination
                for (path, action, risk) in [
                    (from_path, PermissionAction::Read, RiskLevel::Low),
                    (to_path, PermissionAction::Create, RiskLevel::Medium),
                ] {
                    if !permission_manager.check_permission(
                        &self.session_id,
                        "file_operations",
                        &action,
                        Some(path),
                    )? {
                        requests.push(PermissionRequest {
                            id: Uuid::new_v4().to_string(),
                            session_id: self.session_id.clone(),
                            tool_call_id: Some(self.operation_id.clone()),
                            tool_name: "file_operations".to_string(),
                            action,
                            path: Some(path.clone()),
                            description: description.clone(),
                            justification: "Required for atomic file operation".to_string(),
                            risk_level: risk,
                        });
                    }
                }
            }
        }

        Ok(requests)
    }

    /// Check for conflicts between operations
    fn check_operation_conflicts(&self) -> Result<(), Vec<anyhow::Error>> {
        let mut errors = Vec::new();
        let mut file_operations: HashMap<PathBuf, Vec<usize>> = HashMap::new();

        // Track which operations affect each file
        for (index, operation) in self.operations.iter().enumerate() {
            let paths = self.get_operation_paths(operation);
            for path in paths {
                file_operations.entry(path).or_insert_with(Vec::new).push(index);
            }
        }

        // Check for conflicts
        for (path, operation_indices) in file_operations {
            if operation_indices.len() > 1 {
                // Multiple operations on same file - check if they're compatible
                let operations: Vec<&FileOperation> = operation_indices
                    .iter()
                    .map(|&i| &self.operations[i])
                    .collect();

                if !self.operations_are_compatible(&operations) {
                    errors.push(anyhow!(
                        "Conflicting operations on file {}: operations {}",
                        path.display(),
                        operation_indices.iter()
                            .map(|i| (i + 1).to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Check if operations are compatible
    fn operations_are_compatible(&self, operations: &[&FileOperation]) -> bool {
        // For now, only allow one operation per file
        // Future enhancement: allow compatible operation sequences
        operations.len() <= 1
    }

    /// Get all paths affected by an operation
    fn get_operation_paths(&self, operation: &FileOperation) -> Vec<PathBuf> {
        match operation {
            FileOperation::Create { path, .. } |
            FileOperation::Modify { path, .. } |
            FileOperation::Delete { path, .. } => vec![path.clone()],
            FileOperation::Move { from_path, to_path, .. } |
            FileOperation::Copy { from_path, to_path, .. } => vec![from_path.clone(), to_path.clone()],
        }
    }

    /// Create rollback data for all operations
    async fn create_rollback_data(&mut self) -> Result<()> {
        for operation in &self.operations {
            let rollback_data = match operation {
                FileOperation::Create { path, .. } => {
                    RollbackData {
                        operation_id: Uuid::new_v4().to_string(),
                        operation_type: OperationType::Create,
                        target_path: path.clone(),
                        backup_path: None,
                        original_existed: false,
                    }
                }
                FileOperation::Modify { path, .. } => {
                    let backup_path = self.temp_dir.join(format!("backup_{}", Uuid::new_v4()));
                    if path.exists() {
                        fs::copy(path, &backup_path).await?;
                    }
                    RollbackData {
                        operation_id: Uuid::new_v4().to_string(),
                        operation_type: OperationType::Modify,
                        target_path: path.clone(),
                        backup_path: Some(backup_path),
                        original_existed: path.exists(),
                    }
                }
                FileOperation::Delete { path, .. } => {
                    let backup_path = self.temp_dir.join(format!("backup_{}", Uuid::new_v4()));
                    if path.exists() {
                        fs::copy(path, &backup_path).await?;
                    }
                    RollbackData {
                        operation_id: Uuid::new_v4().to_string(),
                        operation_type: OperationType::Delete,
                        target_path: path.clone(),
                        backup_path: Some(backup_path),
                        original_existed: path.exists(),
                    }
                }
                FileOperation::Move { from_path, to_path, .. } => {
                    // For move operations, we backup the source
                    let backup_path = self.temp_dir.join(format!("backup_{}", Uuid::new_v4()));
                    if from_path.exists() {
                        fs::copy(from_path, &backup_path).await?;
                    }
                    RollbackData {
                        operation_id: Uuid::new_v4().to_string(),
                        operation_type: OperationType::Move,
                        target_path: from_path.clone(),
                        backup_path: Some(backup_path),
                        original_existed: from_path.exists(),
                    }
                }
                FileOperation::Copy { from_path, to_path, .. } => {
                    // For copy operations, we backup the destination if it exists
                    let backup_path = if to_path.exists() {
                        let backup = self.temp_dir.join(format!("backup_{}", Uuid::new_v4()));
                        fs::copy(to_path, &backup).await?;
                        Some(backup)
                    } else {
                        None
                    };
                    RollbackData {
                        operation_id: Uuid::new_v4().to_string(),
                        operation_type: OperationType::Copy,
                        target_path: to_path.clone(),
                        backup_path,
                        original_existed: to_path.exists(),
                    }
                }
            };
            self.rollback_data.push(rollback_data);
        }
        Ok(())
    }

    /// Execute a single operation
    async fn execute_single_operation(
        &self,
        operation: &FileOperation,
        file_safety: &mut FileSafetyManager,
    ) -> Result<SafeWriteOperation> {
        match operation {
            FileOperation::Create { path, content, .. } => {
                file_safety.safe_create_file(path, content).await
            }
            FileOperation::Modify { path, old_content, new_content, .. } => {
                file_safety.safe_write_file(path, old_content, new_content).await
            }
            FileOperation::Delete { path, .. } => {
                // Read file first to establish tracking
                let _content = file_safety.safe_read_file(path).await?;
                
                // Delete the file
                fs::remove_file(path).await?;
                
                // Create a dummy SafeWriteOperation for consistency
                Ok(SafeWriteOperation {
                    operation_id: Uuid::new_v4().to_string(),
                    target_path: path.clone(),
                    temp_path: PathBuf::new(),
                    backup_path: None,
                    original_content_hash: None,
                })
            }
            FileOperation::Move { from_path, to_path, .. } => {
                // Read source file
                let content = file_safety.safe_read_file(from_path).await?;
                
                // Create at destination
                let safe_op = file_safety.safe_create_file(to_path, &content).await?;
                
                // Remove source
                fs::remove_file(from_path).await?;
                
                Ok(safe_op)
            }
            FileOperation::Copy { from_path, to_path, .. } => {
                // Read source file
                let content = file_safety.safe_read_file(from_path).await?;
                
                // Create/overwrite at destination
                if to_path.exists() {
                    let old_content = fs::read_to_string(to_path).await?;
                    file_safety.safe_write_file(to_path, &old_content, &content).await
                } else {
                    file_safety.safe_create_file(to_path, &content).await
                }
            }
        }
    }

    /// Rollback all completed operations
    pub async fn rollback(&mut self, file_safety: &mut FileSafetyManager) -> Result<()> {
        // Rollback in reverse order
        for safe_op in self.completed_operations.iter().rev() {
            if let Err(e) = file_safety.rollback_operation(safe_op).await {
                eprintln!("Warning: Failed to rollback operation {}: {}", safe_op.operation_id, e);
            }
        }

        // Restore from rollback data
        for rollback in self.rollback_data.iter().rev() {
            if let Err(e) = self.restore_from_rollback(rollback).await {
                eprintln!("Warning: Failed to restore from rollback {}: {}", rollback.operation_id, e);
            }
        }

        Ok(())
    }

    /// Restore a file from rollback data
    async fn restore_from_rollback(&self, rollback: &RollbackData) -> Result<()> {
        match rollback.operation_type {
            OperationType::Create => {
                // Remove the created file
                if rollback.target_path.exists() {
                    fs::remove_file(&rollback.target_path).await?;
                }
            }
            OperationType::Modify | OperationType::Delete => {
                // Restore from backup
                if let Some(backup_path) = &rollback.backup_path {
                    if backup_path.exists() {
                        fs::copy(backup_path, &rollback.target_path).await?;
                    }
                } else if !rollback.original_existed {
                    // File didn't exist originally, remove it
                    if rollback.target_path.exists() {
                        fs::remove_file(&rollback.target_path).await?;
                    }
                }
            }
            OperationType::Move => {
                // Restore source file and remove destination if created
                if let Some(backup_path) = &rollback.backup_path {
                    if backup_path.exists() {
                        fs::copy(backup_path, &rollback.target_path).await?;
                    }
                }
            }
            OperationType::Copy => {
                // Restore destination if it existed, otherwise remove it
                if let Some(backup_path) = &rollback.backup_path {
                    fs::copy(backup_path, &rollback.target_path).await?;
                } else if !rollback.original_existed && rollback.target_path.exists() {
                    fs::remove_file(&rollback.target_path).await?;
                }
            }
        }
        Ok(())
    }

    /// Clean up temporary files
    async fn cleanup(&self) -> Result<()> {
        if self.temp_dir.exists() {
            fs::remove_dir_all(&self.temp_dir).await?;
        }
        Ok(())
    }

    /// Get operation summary
    pub fn get_summary(&self) -> String {
        format!(
            "Atomic operation {} with {} file operations",
            self.operation_id,
            self.operations.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::permission_manager::PermissionManager;

    #[tokio::test]
    async fn test_atomic_multi_file_create() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        
        let operations = vec![
            FileOperation::Create {
                path: file1.clone(),
                content: "content1".to_string(),
                description: "Create file1".to_string(),
            },
            FileOperation::Create {
                path: file2.clone(),
                content: "content2".to_string(),
                description: "Create file2".to_string(),
            },
        ];

        let atomic_op = AtomicMultiFileOperation::new(operations, "test-session".to_string()).await.unwrap();
        let mut file_safety = FileSafetyManager::new().await.unwrap();
        let perm_storage = temp_dir.path().join("permissions.json");
        let permission_manager = PermissionManager::new(perm_storage).await.unwrap();

        let result = atomic_op.execute(&mut file_safety, &permission_manager).await.unwrap();

        assert!(result.success);
        assert_eq!(result.operations_completed, 2);
        assert!(file1.exists());
        assert!(file2.exists());
        assert_eq!(fs::read_to_string(&file1).await.unwrap(), "content1");
        assert_eq!(fs::read_to_string(&file2).await.unwrap(), "content2");
    }

    #[tokio::test]
    async fn test_atomic_operation_rollback() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("nonexistent/file2.txt"); // This will fail
        
        let operations = vec![
            FileOperation::Create {
                path: file1.clone(),
                content: "content1".to_string(),
                description: "Create file1".to_string(),
            },
            FileOperation::Create {
                path: file2.clone(),
                content: "content2".to_string(),
                description: "Create file2".to_string(),
            },
        ];

        let atomic_op = AtomicMultiFileOperation::new(operations, "test-session".to_string()).await.unwrap();
        let mut file_safety = FileSafetyManager::new().await.unwrap();
        let perm_storage = temp_dir.path().join("permissions.json");
        let permission_manager = PermissionManager::new(perm_storage).await.unwrap();

        let result = atomic_op.execute(&mut file_safety, &permission_manager).await.unwrap();

        assert!(!result.success);
        assert!(!file1.exists()); // Should be rolled back
        assert!(!file2.exists());
    }
}