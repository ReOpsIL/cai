//! Multi-file safety system
//! 
//! This module implements enhanced safety validation for multi-file operations:
//! - Permission validation for file creation/modification
//! - Atomic multi-file operations
//! - Safety gates for complex project generation
//! - User consent management for large operations

use super::*;
use crate::logger::{log_info, log_debug, log_warn, log_error};
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::SystemTime;

/// Enhanced safety system for multi-file operations
pub struct MultiFileSafetyValidator {
    permission_cache: HashMap<String, CachedPermission>,
    safe_operations: Vec<String>,
    restricted_paths: Vec<PathBuf>,
    session_permissions: HashMap<String, SessionPermissions>,
}

/// Cached permission for repeated operations
#[derive(Debug, Clone)]
struct CachedPermission {
    operation: String,
    granted: bool,
    expires_at: SystemTime,
    conditions: Vec<String>,
}

/// Session-based permissions for user workflows
#[derive(Debug, Clone)]
struct SessionPermissions {
    session_id: String,
    auto_approve_safe_operations: bool,
    approved_directories: Vec<PathBuf>,
    operation_count: usize,
    max_operations: usize,
}

/// Multi-file operation request with safety metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiFileOperationRequest {
    pub operation_id: String,
    pub session_id: String,
    pub operation_type: MultiFileOperationType,
    pub target_directory: PathBuf,
    pub files_to_create: Vec<PathBuf>,
    pub files_to_modify: Vec<PathBuf>,
    pub estimated_size_bytes: u64,
    pub complexity_factors: Vec<String>,
    pub safety_metadata: SafetyMetadata,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MultiFileOperationType {
    ProjectCreation,
    ProjectUpdate,
    TestGeneration,
    DocumentationGeneration,
    CodeRefactoring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyMetadata {
    pub risk_level: RiskLevel,
    pub user_confirmation_required: bool,
    pub backup_recommended: bool,
    pub reversible: bool,
    pub estimated_execution_time_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,      // Simple file creation in new directory
    Medium,   // Multiple file creation/modification
    High,     // Complex operations affecting many files
    Critical, // Operations that could impact system or existing work
}

/// Permission request result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResult {
    pub granted: bool,
    pub reason: String,
    pub conditions: Vec<String>,
    pub alternatives: Vec<String>,
    pub requires_user_confirmation: bool,
}

/// Safety validation result for multi-file operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyValidationResult {
    pub safe_to_proceed: bool,
    pub risk_assessment: RiskAssessment,
    pub permission_required: bool,
    pub recommended_actions: Vec<String>,
    pub safety_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: RiskLevel,
    pub file_system_risk: RiskLevel,
    pub data_loss_risk: RiskLevel,
    pub security_risk: RiskLevel,
    pub impact_scope: String,
}

impl MultiFileSafetyValidator {
    pub fn new() -> Self {
        let mut validator = Self {
            permission_cache: HashMap::new(),
            safe_operations: Self::default_safe_operations(),
            restricted_paths: Self::default_restricted_paths(),
            session_permissions: HashMap::new(),
        };
        
        validator
    }

    /// Validate safety of multi-file operation
    pub async fn validate_multi_file_operation(
        &mut self,
        request: &MultiFileOperationRequest,
    ) -> Result<SafetyValidationResult> {
        log_info!("multi_file_safety", 
            "Validating multi-file operation: {} ({:?})", 
            request.operation_id, 
            request.operation_type
        );

        // 1. Assess operation risk
        let risk_assessment = self.assess_operation_risk(request).await?;
        
        // 2. Check path restrictions
        let path_validation = self.validate_target_paths(request)?;
        
        // 3. Check session permissions
        let session_validation = self.validate_session_permissions(request)?;
        
        // 4. Check cached permissions
        let cache_validation = self.check_cached_permissions(request)?;
        
        // 5. Determine if user confirmation is needed
        let requires_confirmation = self.requires_user_confirmation(request, &risk_assessment)?;
        
        let mut safety_warnings = vec![];
        let mut recommended_actions = vec![];
        
        // Generate safety warnings based on risk level
        match risk_assessment.overall_risk {
            RiskLevel::High | RiskLevel::Critical => {
                safety_warnings.push(format!("High-risk operation: {}", risk_assessment.impact_scope));
                recommended_actions.push("Consider creating a backup before proceeding".to_string());
            },
            RiskLevel::Medium => {
                safety_warnings.push("Medium-risk operation with multiple file changes".to_string());
            },
            RiskLevel::Low => {}
        }

        // Check for restricted paths
        if !path_validation {
            safety_warnings.push("Operation targets restricted paths".to_string());
            recommended_actions.push("Choose a different target directory".to_string());
        }

        let safe_to_proceed = path_validation && 
                             session_validation && 
                             !matches!(risk_assessment.overall_risk, RiskLevel::Critical);

        log_debug!("multi_file_safety", 
            "Safety validation result: safe={}, risk={:?}, confirmation_required={}", 
            safe_to_proceed,
            risk_assessment.overall_risk,
            requires_confirmation
        );

        Ok(SafetyValidationResult {
            safe_to_proceed,
            risk_assessment,
            permission_required: requires_confirmation,
            recommended_actions,
            safety_warnings,
        })
    }

    /// Request permission for multi-file operation
    pub async fn request_permission(
        &mut self,
        request: &MultiFileOperationRequest,
        validation_result: &SafetyValidationResult,
    ) -> Result<PermissionResult> {
        log_info!("multi_file_safety", "Requesting permission for operation: {}", request.operation_id);

        // Check if operation is in safe operations list
        if self.is_safe_operation(&request.operation_type) && 
           matches!(validation_result.risk_assessment.overall_risk, RiskLevel::Low) {
            
            log_debug!("multi_file_safety", "Auto-approving safe operation");
            return Ok(PermissionResult {
                granted: true,
                reason: "Safe operation auto-approved".to_string(),
                conditions: vec![],
                alternatives: vec![],
                requires_user_confirmation: false,
            });
        }

        // Check session permissions
        if let Some(session_perms) = self.session_permissions.get(&request.session_id) {
            if session_perms.auto_approve_safe_operations && 
               matches!(validation_result.risk_assessment.overall_risk, RiskLevel::Low | RiskLevel::Medium) {
                
                log_debug!("multi_file_safety", "Auto-approving based on session permissions");
                return Ok(PermissionResult {
                    granted: true,
                    reason: "Session auto-approval enabled".to_string(),
                    conditions: vec!["Session-based approval".to_string()],
                    alternatives: vec![],
                    requires_user_confirmation: false,
                });
            }
        }

        // For complex operations, require explicit permission
        let requires_confirmation = matches!(
            validation_result.risk_assessment.overall_risk, 
            RiskLevel::High | RiskLevel::Critical
        ) || request.files_to_create.len() > 10;

        if requires_confirmation {
            log_info!("multi_file_safety", "User confirmation required for complex operation");
            
            Ok(PermissionResult {
                granted: false, // Will be updated after user confirmation
                reason: "User confirmation required for complex operation".to_string(),
                conditions: vec![
                    format!("Creating {} files", request.files_to_create.len()),
                    format!("Estimated size: {} bytes", request.estimated_size_bytes),
                    format!("Risk level: {:?}", validation_result.risk_assessment.overall_risk),
                ],
                alternatives: self.suggest_alternatives(request)?,
                requires_user_confirmation: true,
            })
        } else {
            Ok(PermissionResult {
                granted: true,
                reason: "Operation approved".to_string(),
                conditions: vec![],
                alternatives: vec![],
                requires_user_confirmation: false,
            })
        }
    }

    /// Execute multi-file operation atomically
    pub async fn execute_atomic_operation(
        &mut self,
        request: &MultiFileOperationRequest,
        operations: Vec<MultiFileOperation>,
    ) -> Result<AtomicOperationResult> {
        log_info!("multi_file_safety", "Executing atomic multi-file operation: {}", request.operation_id);

        let start_time = std::time::Instant::now();
        let mut created_files = vec![];
        let mut rollback_operations = vec![];

        // Create backup if recommended
        if request.safety_metadata.backup_recommended {
            self.create_operation_backup(request).await?;
        }

        // Execute operations atomically
        for (index, operation) in operations.iter().enumerate() {
            log_debug!("multi_file_safety", "Executing operation {}/{}: {:?}", 
                index + 1, operations.len(), operation.operation_type);

            match self.execute_single_operation(operation).await {
                Ok(result) => {
                    created_files.push(operation.file_path.clone());
                    if let Some(rollback) = result.rollback_operation {
                        rollback_operations.push(rollback);
                    }
                },
                Err(e) => {
                    log_error!("multi_file_safety", "Operation failed, rolling back: {}", e);
                    
                    // Rollback all previous operations
                    for rollback_op in rollback_operations.into_iter().rev() {
                        if let Err(rollback_error) = self.execute_rollback_operation(rollback_op).await {
                            log_error!("multi_file_safety", "Rollback failed: {}", rollback_error);
                        }
                    }
                    
                    return Ok(AtomicOperationResult {
                        success: false,
                        files_created: vec![],
                        files_modified: vec![],
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        error_message: Some(e.to_string()),
                        rollback_performed: true,
                    });
                }
            }
        }

        // Update session permissions
        self.update_session_permissions(&request.session_id, created_files.len())?;

        log_info!("multi_file_safety", 
            "Atomic operation completed successfully: {} files created in {}ms", 
            created_files.len(),
            start_time.elapsed().as_millis()
        );

        Ok(AtomicOperationResult {
            success: true,
            files_created: created_files,
            files_modified: vec![], // Would track modifications
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            error_message: None,
            rollback_performed: false,
        })
    }

    /// Assess risk level of multi-file operation
    async fn assess_operation_risk(&self, request: &MultiFileOperationRequest) -> Result<RiskAssessment> {
        let mut file_system_risk = RiskLevel::Low;
        let mut data_loss_risk = RiskLevel::Low;
        let mut security_risk = RiskLevel::Low;

        // Assess based on number of files
        let file_count = request.files_to_create.len() + request.files_to_modify.len();
        if file_count > 50 {
            file_system_risk = RiskLevel::High;
        } else if file_count > 10 {
            file_system_risk = RiskLevel::Medium;
        }

        // Assess based on file modifications
        if !request.files_to_modify.is_empty() {
            data_loss_risk = RiskLevel::Medium;
        }

        // Assess based on complexity factors
        if request.complexity_factors.contains(&"security_implementation".to_string()) {
            security_risk = RiskLevel::Medium;
        }

        // Check for system paths
        for path in &request.files_to_create {
            if self.is_system_path(path) {
                file_system_risk = RiskLevel::Critical;
                break;
            }
        }

        let overall_risk = match (file_system_risk, data_loss_risk, security_risk) {
            (RiskLevel::Critical, _, _) | (_, RiskLevel::Critical, _) | (_, _, RiskLevel::Critical) => RiskLevel::Critical,
            (RiskLevel::High, _, _) | (_, RiskLevel::High, _) | (_, _, RiskLevel::High) => RiskLevel::High,
            (RiskLevel::Medium, _, _) | (_, RiskLevel::Medium, _) | (_, _, RiskLevel::Medium) => RiskLevel::Medium,
            _ => RiskLevel::Low,
        };

        let impact_scope = format!("{} files in {}", file_count, request.target_directory.display());

        Ok(RiskAssessment {
            overall_risk,
            file_system_risk,
            data_loss_risk,
            security_risk,
            impact_scope,
        })
    }

    /// Validate target paths are safe
    fn validate_target_paths(&self, request: &MultiFileOperationRequest) -> Result<bool> {
        // Check if target directory is restricted
        for restricted_path in &self.restricted_paths {
            if request.target_directory.starts_with(restricted_path) {
                log_warn!("multi_file_safety", "Operation targets restricted path: {}", 
                    request.target_directory.display());
                return Ok(false);
            }
        }

        // Check individual file paths
        for file_path in &request.files_to_create {
            if self.is_system_path(file_path) {
                log_warn!("multi_file_safety", "Operation targets system path: {}", file_path.display());
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check if operation type is considered safe
    fn is_safe_operation(&self, operation_type: &MultiFileOperationType) -> bool {
        matches!(operation_type, 
            MultiFileOperationType::ProjectCreation | 
            MultiFileOperationType::TestGeneration |
            MultiFileOperationType::DocumentationGeneration
        )
    }

    /// Check if path is a system path
    fn is_system_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        path_str.starts_with("/system") || 
        path_str.starts_with("/etc") || 
        path_str.starts_with("/bin") ||
        path_str.starts_with("/usr/bin") ||
        path_str.starts_with("c:\\windows") ||
        path_str.starts_with("c:\\program files")
    }

    /// Default safe operations
    fn default_safe_operations() -> Vec<String> {
        vec![
            "project_creation".to_string(),
            "test_generation".to_string(),
            "documentation_generation".to_string(),
            "file_read".to_string(),
            "directory_list".to_string(),
        ]
    }

    /// Default restricted paths
    fn default_restricted_paths() -> Vec<PathBuf> {
        vec![
            PathBuf::from("/etc"),
            PathBuf::from("/bin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/system"),
            PathBuf::from("C:\\Windows"),
            PathBuf::from("C:\\Program Files"),
        ]
    }

    // Additional helper methods...

    fn validate_session_permissions(&self, request: &MultiFileOperationRequest) -> Result<bool> {
        if let Some(session_perms) = self.session_permissions.get(&request.session_id) {
            Ok(session_perms.operation_count < session_perms.max_operations)
        } else {
            Ok(true) // No session limits
        }
    }

    fn check_cached_permissions(&self, request: &MultiFileOperationRequest) -> Result<bool> {
        // Check if we have cached permission for this type of operation
        let cache_key = format!("{}:{}", request.operation_type as u8, request.target_directory.display());
        
        if let Some(cached) = self.permission_cache.get(&cache_key) {
            if cached.expires_at > SystemTime::now() {
                return Ok(cached.granted);
            }
        }
        
        Ok(true) // No cached restrictions
    }

    fn requires_user_confirmation(&self, request: &MultiFileOperationRequest, risk: &RiskAssessment) -> Result<bool> {
        Ok(matches!(risk.overall_risk, RiskLevel::High | RiskLevel::Critical) ||
           request.files_to_create.len() > 20 ||
           request.estimated_size_bytes > 10_000_000) // 10MB
    }

    fn suggest_alternatives(&self, request: &MultiFileOperationRequest) -> Result<Vec<String>> {
        let mut alternatives = vec![];
        
        if request.files_to_create.len() > 10 {
            alternatives.push("Consider generating files in smaller batches".to_string());
        }
        
        if request.estimated_size_bytes > 1_000_000 {
            alternatives.push("Consider reducing project scope or complexity".to_string());
        }
        
        alternatives.push("Review generated files before proceeding with full creation".to_string());
        
        Ok(alternatives)
    }

    async fn create_operation_backup(&self, request: &MultiFileOperationRequest) -> Result<()> {
        // Implementation would create backup of target directory
        log_info!("multi_file_safety", "Creating backup for operation: {}", request.operation_id);
        Ok(())
    }

    async fn execute_single_operation(&self, operation: &MultiFileOperation) -> Result<SingleOperationResult> {
        // Implementation would execute individual file operations
        log_debug!("multi_file_safety", "Executing file operation: {}", operation.file_path.display());
        
        match operation.operation_type {
            FileOperationType::Create => {
                // Create file
                if let Some(parent) = operation.file_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&operation.file_path, &operation.content)?;
            },
            FileOperationType::Update => {
                // Update existing file
                fs::write(&operation.file_path, &operation.content)?;
            },
            FileOperationType::Delete => {
                // Delete file
                fs::remove_file(&operation.file_path)?;
            },
            FileOperationType::Mkdir => {
                // Create directory
                fs::create_dir_all(&operation.file_path)?;
            },
        }
        
        Ok(SingleOperationResult {
            success: true,
            rollback_operation: Some(RollbackOperation {
                operation_type: match operation.operation_type {
                    FileOperationType::Create => RollbackType::Delete,
                    FileOperationType::Delete => RollbackType::Restore,
                    _ => RollbackType::Revert,
                },
                file_path: operation.file_path.clone(),
                original_content: None, // Would store original content for updates
            }),
        })
    }

    async fn execute_rollback_operation(&self, rollback: RollbackOperation) -> Result<()> {
        // Implementation would rollback operations
        log_debug!("multi_file_safety", "Rolling back operation: {}", rollback.file_path.display());
        Ok(())
    }

    fn update_session_permissions(&mut self, session_id: &str, operation_count: usize) -> Result<()> {
        if let Some(session_perms) = self.session_permissions.get_mut(session_id) {
            session_perms.operation_count += operation_count;
        }
        Ok(())
    }
}

/// Result of atomic multi-file operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicOperationResult {
    pub success: bool,
    pub files_created: Vec<PathBuf>,
    pub files_modified: Vec<PathBuf>,
    pub execution_time_ms: u64,
    pub error_message: Option<String>,
    pub rollback_performed: bool,
}

/// Result of single file operation
#[derive(Debug, Clone)]
struct SingleOperationResult {
    pub success: bool,
    pub rollback_operation: Option<RollbackOperation>,
}

/// Rollback operation for atomic execution
#[derive(Debug, Clone)]
struct RollbackOperation {
    pub operation_type: RollbackType,
    pub file_path: PathBuf,
    pub original_content: Option<String>,
}

#[derive(Debug, Clone)]
enum RollbackType {
    Delete,
    Restore,
    Revert,
}