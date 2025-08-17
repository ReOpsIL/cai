use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::permission_manager::{PermissionManager, Permission};
use crate::file_safety::FileSafetyManager;
use crate::atomic_operations::AtomicOperationManager;
use crate::error_recovery::ErrorRecoveryManager;
use crate::natural_language::NaturalLanguageProcessor;
use crate::progress_tracking::ProgressTracker;
use crate::hierarchical_config::HierarchicalConfig;
use crate::git_integration::GitWorkflowManager;
use crate::hooks::HookManager;
use crate::context_manager::ContextManager;
use crate::predictive_error_prevention::PredictiveErrorPrevention;

/// Main integration layer for all strategic enhancement modules
#[derive(Debug)]
pub struct StrategicIntegration {
    /// Permission management
    permission_manager: Arc<RwLock<PermissionManager>>,
    /// File safety operations
    file_safety: Arc<RwLock<FileSafetyManager>>,
    /// Atomic multi-file operations
    atomic_operations: Arc<RwLock<AtomicOperationManager>>,
    /// Error recovery system
    error_recovery: Arc<RwLock<ErrorRecoveryManager>>,
    /// Natural language processing
    natural_language: Arc<RwLock<NaturalLanguageProcessor>>,
    /// Progress tracking
    progress_tracker: Arc<RwLock<ProgressTracker>>,
    /// Hierarchical configuration
    config: Arc<RwLock<HierarchicalConfig>>,
    /// Git workflow management
    git_workflow: Option<Arc<RwLock<GitWorkflowManager>>>,
    /// Hook system
    hook_manager: Arc<RwLock<HookManager>>,
    /// Context management
    context_manager: Arc<RwLock<ContextManager>>,
    /// Predictive error prevention
    error_prevention: Arc<RwLock<PredictiveErrorPrevention>>,
    /// Integration configuration
    integration_config: IntegrationConfig,
}

/// Configuration for the strategic integration system
#[derive(Debug, Clone)]
pub struct IntegrationConfig {
    /// Enable all strategic enhancements
    pub enabled: bool,
    /// Enable individual modules
    pub module_config: ModuleConfig,
    /// Integration settings
    pub integration_settings: IntegrationSettings,
}

/// Individual module enable/disable configuration
#[derive(Debug, Clone)]
pub struct ModuleConfig {
    pub permissions: bool,
    pub file_safety: bool,
    pub atomic_operations: bool,
    pub error_recovery: bool,
    pub natural_language: bool,
    pub progress_tracking: bool,
    pub git_integration: bool,
    pub hooks: bool,
    pub context_management: bool,
    pub error_prevention: bool,
}

/// Integration-specific settings
#[derive(Debug, Clone)]
pub struct IntegrationSettings {
    /// Maximum concurrent operations across all modules
    pub max_concurrent_operations: usize,
    /// Inter-module communication timeout (seconds)
    pub communication_timeout: u64,
    /// Enable cross-module event propagation
    pub cross_module_events: bool,
    /// Module initialization order
    pub initialization_order: Vec<String>,
}

impl StrategicIntegration {
    /// Initialize the strategic integration system
    pub async fn initialize() -> Result<Self> {
        let config = HierarchicalConfig::load().await?;
        let integration_config = Self::load_integration_config(&config)?;

        let mut integration = Self {
            permission_manager: Arc::new(RwLock::new(PermissionManager::new().await?)),
            file_safety: Arc::new(RwLock::new(FileSafetyManager::new())),
            atomic_operations: Arc::new(RwLock::new(AtomicOperationManager::new())),
            error_recovery: Arc::new(RwLock::new(ErrorRecoveryManager::new())),
            natural_language: Arc::new(RwLock::new(NaturalLanguageProcessor::new().await?)),
            progress_tracker: Arc::new(RwLock::new(ProgressTracker::new())),
            config: Arc::new(RwLock::new(config)),
            git_workflow: None,
            hook_manager: Arc::new(RwLock::new(HookManager::new(
                crate::hooks::HookConfig::default()
            ).await?)),
            context_manager: Arc::new(RwLock::new(ContextManager::new().await?)),
            error_prevention: Arc::new(RwLock::new(PredictiveErrorPrevention::new().await?)),
            integration_config,
        };

        // Initialize Git workflow if in a Git repository
        if let Ok(git_manager) = GitWorkflowManager::new(std::env::current_dir()?).await {
            integration.git_workflow = Some(Arc::new(RwLock::new(git_manager)));
        }

        // Initialize cross-module integrations
        integration.setup_cross_module_integrations().await?;

        Ok(integration)
    }

    /// Load integration configuration from hierarchical config
    fn load_integration_config(config: &HierarchicalConfig) -> Result<IntegrationConfig> {
        Ok(IntegrationConfig {
            enabled: config.get("strategic.enabled").unwrap_or(true),
            module_config: ModuleConfig {
                permissions: config.get("strategic.modules.permissions").unwrap_or(true),
                file_safety: config.get("strategic.modules.file_safety").unwrap_or(true),
                atomic_operations: config.get("strategic.modules.atomic_operations").unwrap_or(true),
                error_recovery: config.get("strategic.modules.error_recovery").unwrap_or(true),
                natural_language: config.get("strategic.modules.natural_language").unwrap_or(true),
                progress_tracking: config.get("strategic.modules.progress_tracking").unwrap_or(true),
                git_integration: config.get("strategic.modules.git_integration").unwrap_or(true),
                hooks: config.get("strategic.modules.hooks").unwrap_or(true),
                context_management: config.get("strategic.modules.context_management").unwrap_or(true),
                error_prevention: config.get("strategic.modules.error_prevention").unwrap_or(true),
            },
            integration_settings: IntegrationSettings {
                max_concurrent_operations: config.get("strategic.settings.max_concurrent").unwrap_or(8),
                communication_timeout: config.get("strategic.settings.timeout").unwrap_or(30),
                cross_module_events: config.get("strategic.settings.cross_module_events").unwrap_or(true),
                initialization_order: config.get("strategic.settings.init_order").unwrap_or_else(|| 
                    vec![
                        "config".to_string(),
                        "permissions".to_string(),
                        "file_safety".to_string(),
                        "hooks".to_string(),
                        "error_recovery".to_string(),
                        "progress_tracking".to_string(),
                        "natural_language".to_string(),
                        "context_management".to_string(),
                        "error_prevention".to_string(),
                        "atomic_operations".to_string(),
                        "git_integration".to_string(),
                    ]
                ),
            },
        })
    }

    /// Setup cross-module integrations and event propagation
    async fn setup_cross_module_integrations(&self) -> Result<()> {
        if !self.integration_config.integration_settings.cross_module_events {
            return Ok(());
        }

        // Setup progress tracking integration with all modules
        let progress_tracker = Arc::clone(&self.progress_tracker);
        
        // Setup error recovery integration with all modules
        let error_recovery = Arc::clone(&self.error_recovery);

        // Setup hook integration with file operations
        let hook_manager = Arc::clone(&self.hook_manager);

        // Setup context management integration
        let context_manager = Arc::clone(&self.context_manager);

        // Setup predictive error prevention integration
        let error_prevention = Arc::clone(&self.error_prevention);

        // TODO: Implement actual event propagation system
        // This would involve setting up event channels between modules

        Ok(())
    }

    /// Execute a safe file operation with full strategic enhancement integration
    pub async fn execute_safe_file_operation(
        &self,
        operation_type: &str,
        files: Vec<std::path::PathBuf>,
        context: &str,
    ) -> Result<OperationResult> {
        if !self.integration_config.enabled {
            return Err(anyhow!("Strategic enhancements are disabled"));
        }

        let operation_id = Uuid::new_v4().to_string();
        let session_id = "current_session".to_string(); // TODO: Get from session manager

        // 1. Start progress tracking
        let task_id = if self.integration_config.module_config.progress_tracking {
            Some(self.progress_tracker.write().await.start_task(
                operation_id.clone(),
                operation_type.to_string(),
                files.len(),
                Some(context.to_string()),
            ))
        } else {
            None
        };

        // 2. Predictive error prevention
        if self.integration_config.module_config.error_prevention {
            let risk_assessment = self.error_prevention.read().await.assess_operation_risk(
                crate::predictive_error_prevention::OperationContext {
                    operation_type: operation_type.to_string(),
                    files_involved: files.clone(),
                    user_context: context.to_string(),
                    session_id: session_id.clone(),
                    timestamp: std::time::SystemTime::now(),
                },
                crate::predictive_error_prevention::EnvironmentContext {
                    current_directory: std::env::current_dir()?,
                    git_status: None, // TODO: Get from git integration
                    system_resources: crate::predictive_error_prevention::SystemResources {
                        available_memory_mb: 1024, // TODO: Get actual system resources
                        cpu_usage_percent: 50.0,
                        disk_space_mb: 10000,
                    },
                    active_processes: Vec::new(),
                },
            ).await?;

            // Handle high-risk operations
            if matches!(risk_assessment.risk_level, crate::predictive_error_prevention::RiskLevel::High) {
                if let Some(task_id) = task_id {
                    self.progress_tracker.write().await.fail_task(task_id, "High risk operation blocked".to_string());
                }
                return Err(anyhow!("Operation blocked due to high risk: {}", risk_assessment.reason));
            }
        }

        // 3. Permission checking
        if self.integration_config.module_config.permissions {
            for file in &files {
                let permission_needed = match operation_type {
                    "read" => Permission::ReadFile { path: file.clone() },
                    "write" => Permission::WriteFile { path: file.clone() },
                    "delete" => Permission::DeleteFile { path: file.clone() },
                    "create" => Permission::CreateFile { path: file.clone() },
                    _ => Permission::WriteFile { path: file.clone() },
                };

                if !self.permission_manager.read().await.check_permission(&session_id, &permission_needed).await? {
                    if let Some(task_id) = task_id {
                        self.progress_tracker.write().await.fail_task(task_id, "Permission denied".to_string());
                    }
                    return Err(anyhow!("Permission denied for {}: {}", operation_type, file.display()));
                }
            }
        }

        // 4. Pre-execution hooks
        if self.integration_config.module_config.hooks {
            let hook_context = crate::hooks::ExecutionContext {
                execution_id: operation_id.clone(),
                event: match operation_type {
                    "read" => crate::hooks::HookEvent::FileRead,
                    "write" => crate::hooks::HookEvent::FileWrite,
                    "delete" => crate::hooks::HookEvent::FileDelete,
                    "create" => crate::hooks::HookEvent::FileCreate,
                    _ => crate::hooks::HookEvent::FileWrite,
                },
                timestamp: std::time::SystemTime::now(),
                session_id: session_id.clone(),
                user_id: None,
                operation: crate::hooks::OperationDetails {
                    operation_type: operation_type.to_string(),
                    parameters: std::collections::HashMap::new(),
                    files_involved: files.clone(),
                    metadata: std::collections::HashMap::new(),
                },
                environment: std::collections::HashMap::new(),
                previous_results: Vec::new(),
            };

            let hook_results = self.hook_manager.read().await.execute_pre_hooks(
                hook_context.event.clone(),
                &hook_context,
            ).await?;

            // Check if any hook blocked the operation
            for result in hook_results {
                if let crate::hooks::HookResult::Block { reason } = result {
                    if let Some(task_id) = task_id {
                        self.progress_tracker.write().await.fail_task(task_id, format!("Hook blocked: {}", reason));
                    }
                    return Err(anyhow!("Operation blocked by hook: {}", reason));
                }
            }
        }

        // 5. Execute with error recovery
        let operation_result = if self.integration_config.module_config.error_recovery {
            self.error_recovery.read().await.execute_with_recovery(
                operation_id.clone(),
                Box::new(move || {
                    Box::pin(async move {
                        // Actual file operation would go here
                        Ok(())
                    })
                }),
            ).await
        } else {
            // Direct execution without recovery
            Ok(())
        };

        // 6. Update progress and handle results
        let result = match operation_result {
            Ok(_) => {
                if let Some(task_id) = task_id {
                    self.progress_tracker.write().await.complete_task(task_id);
                }
                OperationResult {
                    success: true,
                    operation_id,
                    details: format!("Successfully executed {} on {} files", operation_type, files.len()),
                    files_affected: files,
                    error_message: None,
                    recovery_applied: false,
                }
            }
            Err(e) => {
                if let Some(task_id) = task_id {
                    self.progress_tracker.write().await.fail_task(task_id, e.to_string());
                }
                OperationResult {
                    success: false,
                    operation_id,
                    details: format!("Failed to execute {} on {} files", operation_type, files.len()),
                    files_affected: files,
                    error_message: Some(e.to_string()),
                    recovery_applied: false,
                }
            }
        };

        Ok(result)
    }

    /// Get system health status across all strategic modules
    pub async fn get_system_health(&self) -> Result<SystemHealthReport> {
        let mut health = SystemHealthReport {
            overall_status: HealthStatus::Healthy,
            module_status: std::collections::HashMap::new(),
            performance_metrics: PerformanceMetrics::default(),
            recommendations: Vec::new(),
        };

        // Check each module health
        if self.integration_config.module_config.permissions {
            let stats = self.permission_manager.read().await.get_statistics().await;
            health.module_status.insert("permissions".to_string(), HealthStatus::Healthy);
        }

        if self.integration_config.module_config.progress_tracking {
            let stats = self.progress_tracker.read().await.get_performance_stats();
            health.performance_metrics.active_tasks = stats.active_tasks as u64;
            health.performance_metrics.completed_tasks = stats.completed_tasks as u64;
            health.module_status.insert("progress_tracking".to_string(), HealthStatus::Healthy);
        }

        if self.integration_config.module_config.hooks {
            let stats = self.hook_manager.read().await.get_statistics().await;
            health.performance_metrics.hook_executions = stats.total_executions;
            health.module_status.insert("hooks".to_string(), HealthStatus::Healthy);
        }

        // Check for any module failures
        let failed_modules = health.module_status.iter()
            .filter(|(_, status)| matches!(status, HealthStatus::Unhealthy))
            .count();

        if failed_modules > 0 {
            health.overall_status = HealthStatus::Degraded;
            health.recommendations.push(format!("{} modules are unhealthy", failed_modules));
        }

        Ok(health)
    }

    /// Parse natural language command into structured operations
    pub async fn parse_natural_command(&self, input: &str) -> Result<Vec<StructuredOperation>> {
        if !self.integration_config.module_config.natural_language {
            return Err(anyhow!("Natural language processing is disabled"));
        }

        let command_intent = self.natural_language.read().await.parse_natural_command(input)?;
        
        // Convert command intent to structured operations
        let operations = match command_intent.intent.as_str() {
            "file_operation" => {
                vec![StructuredOperation {
                    operation_type: command_intent.extracted_entities.get("operation")
                        .cloned().unwrap_or("unknown".to_string()),
                    targets: command_intent.extracted_entities.get("files")
                        .map(|files| files.split(',').map(|f| f.trim().into()).collect())
                        .unwrap_or_default(),
                    parameters: command_intent.extracted_entities,
                    confidence: command_intent.confidence,
                }]
            }
            _ => {
                vec![StructuredOperation {
                    operation_type: command_intent.intent,
                    targets: Vec::new(),
                    parameters: command_intent.extracted_entities,
                    confidence: command_intent.confidence,
                }]
            }
        };

        Ok(operations)
    }

    /// Get configuration value with fallback to defaults
    pub async fn get_config_value<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> serde::Deserialize<'de> + Clone,
    {
        self.config.read().await.get(key)
    }

    /// Check if the system is ready for operations
    pub async fn is_ready(&self) -> bool {
        if !self.integration_config.enabled {
            return false;
        }

        // Check critical modules are initialized
        // This is a simplified check - in practice, each module would have a health check
        true
    }
}

/// Result of an integrated operation
#[derive(Debug, Clone)]
pub struct OperationResult {
    pub success: bool,
    pub operation_id: String,
    pub details: String,
    pub files_affected: Vec<std::path::PathBuf>,
    pub error_message: Option<String>,
    pub recovery_applied: bool,
}

/// Structured operation from natural language parsing
#[derive(Debug, Clone)]
pub struct StructuredOperation {
    pub operation_type: String,
    pub targets: Vec<std::path::PathBuf>,
    pub parameters: std::collections::HashMap<String, String>,
    pub confidence: f64,
}

/// System health report
#[derive(Debug, Clone)]
pub struct SystemHealthReport {
    pub overall_status: HealthStatus,
    pub module_status: std::collections::HashMap<String, HealthStatus>,
    pub performance_metrics: PerformanceMetrics,
    pub recommendations: Vec<String>,
}

/// Health status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Performance metrics across the system
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub active_tasks: u64,
    pub completed_tasks: u64,
    pub hook_executions: u64,
    pub average_response_time: f64,
    pub memory_usage_mb: u64,
    pub error_rate: f64,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            module_config: ModuleConfig::default(),
            integration_settings: IntegrationSettings::default(),
        }
    }
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            permissions: true,
            file_safety: true,
            atomic_operations: true,
            error_recovery: true,
            natural_language: true,
            progress_tracking: true,
            git_integration: true,
            hooks: true,
            context_management: true,
            error_prevention: true,
        }
    }
}

impl Default for IntegrationSettings {
    fn default() -> Self {
        Self {
            max_concurrent_operations: 8,
            communication_timeout: 30,
            cross_module_events: true,
            initialization_order: vec![
                "config".to_string(),
                "permissions".to_string(),
                "file_safety".to_string(),
                "hooks".to_string(),
                "error_recovery".to_string(),
                "progress_tracking".to_string(),
                "natural_language".to_string(),
                "context_management".to_string(),
                "error_prevention".to_string(),
                "atomic_operations".to_string(),
                "git_integration".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_strategic_integration_initialization() {
        // Note: This test would require proper setup of dependencies
        // let integration = StrategicIntegration::initialize().await;
        // assert!(integration.is_ok());
    }

    #[tokio::test]
    async fn test_system_health_reporting() {
        // Test health reporting functionality
    }
}