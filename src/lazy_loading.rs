use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::logger::{log_debug, log_info, log_warn};

/// Lazy loading manager for CAI enhancement systems
/// Loads expensive components only when needed for specific commands
pub struct LazyLoader {
    /// Component load status tracking
    loaded_components: Arc<Mutex<std::collections::HashSet<Component>>>,
}

/// Available components that can be lazily loaded
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Component {
    ErrorRecovery,
    PerformanceOptimization,
    WorkflowStateManagement,
    PredictiveErrorPrevention,
    ContextAwareExecution,
    TestInfrastructure,
    EdgeCaseMastery,
    McpServers,
    PromptManager,
    OpenRouterClient,
    WorkflowOrchestrator,
    TaskExecutor,
    ChatInterface,
}

/// Commands and their required components
#[derive(Debug, Clone)]
pub struct CommandRequirements {
    pub command: String,
    pub required_components: Vec<Component>,
    pub optional_components: Vec<Component>,
}

impl LazyLoader {
    pub fn new() -> Self {
        Self {
            loaded_components: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }

    /// Get command requirements based on the command being executed
    pub fn get_command_requirements(command: &str) -> CommandRequirements {
        match command {
            // Chat command needs most systems for full functionality
            "chat" => CommandRequirements {
                command: command.to_string(),
                required_components: vec![
                    Component::PromptManager,
                    Component::OpenRouterClient,
                    Component::WorkflowOrchestrator,
                    Component::TaskExecutor,
                    Component::ChatInterface,
                ],
                optional_components: vec![
                    Component::ErrorRecovery,
                    Component::PerformanceOptimization,
                    Component::WorkflowStateManagement,
                    Component::PredictiveErrorPrevention,
                    Component::ContextAwareExecution,
                ],
            },
            
            // Workflow commands need workflow-specific systems
            "workflow" => CommandRequirements {
                command: command.to_string(),
                required_components: vec![
                    Component::WorkflowOrchestrator,
                    Component::TaskExecutor,
                    Component::WorkflowStateManagement,
                ],
                optional_components: vec![
                    Component::ErrorRecovery,
                    Component::PerformanceOptimization,
                    Component::PredictiveErrorPrevention,
                    Component::ContextAwareExecution,
                    Component::EdgeCaseMastery,
                ],
            },
            
            // MCP commands need MCP systems
            "mcp" => CommandRequirements {
                command: command.to_string(),
                required_components: vec![
                    Component::McpServers,
                    Component::TaskExecutor,
                ],
                optional_components: vec![
                    Component::ErrorRecovery,
                    Component::PerformanceOptimization,
                ],
            },
            
            // Scan command might need various systems
            "scan" => CommandRequirements {
                command: command.to_string(),
                required_components: vec![
                    Component::PromptManager,
                ],
                optional_components: vec![
                    Component::PerformanceOptimization,
                    Component::ErrorRecovery,
                ],
            },
            
            // Task demo needs test infrastructure
            "task-demo" => CommandRequirements {
                command: command.to_string(),
                required_components: vec![
                    Component::TestInfrastructure,
                    Component::TaskExecutor,
                    Component::McpServers,
                ],
                optional_components: vec![
                    Component::ErrorRecovery,
                    Component::PerformanceOptimization,
                    Component::EdgeCaseMastery,
                ],
            },
            
            // Basic commands handled by fast path don't need any components
            _ => CommandRequirements {
                command: command.to_string(),
                required_components: vec![],
                optional_components: vec![],
            },
        }
    }

    /// Load components required for a specific command
    pub async fn load_for_command(&self, command: &str) -> Result<()> {
        let requirements = Self::get_command_requirements(command);
        
        log_info!("lazy_loader", "🔄 Loading components for command: {}", command);
        
        // Load required components
        for component in &requirements.required_components {
            self.load_component(component).await?;
        }
        
        // Load optional components (with graceful failure)
        for component in &requirements.optional_components {
            if let Err(e) = self.load_component(component).await {
                log_warn!("lazy_loader", "⚠️ Optional component {:?} failed to load: {}", component, e);
            }
        }
        
        log_info!("lazy_loader", "✅ Component loading completed for command: {}", command);
        Ok(())
    }

    /// Load a specific component if not already loaded
    pub async fn load_component(&self, component: &Component) -> Result<()> {
        let mut loaded = self.loaded_components.lock().await;
        
        if loaded.contains(component) {
            log_debug!("lazy_loader", "Component {:?} already loaded, skipping", component);
            return Ok(());
        }
        
        log_debug!("lazy_loader", "🔄 Loading component: {:?}", component);
        
        match component {
            Component::ErrorRecovery => {
                crate::advanced_error_recovery::initialize_error_recovery(
                    crate::advanced_error_recovery::ErrorRecoveryConfig::default()
                ).await?;
                log_info!("lazy_loader", "✅ Error recovery system loaded");
            },
            
            Component::PerformanceOptimization => {
                crate::performance_optimization::initialize_performance_optimizer(
                    crate::performance_optimization::PerformanceConfig::default()
                ).await?;
                log_info!("lazy_loader", "✅ Performance optimization system loaded");
            },
            
            Component::WorkflowStateManagement => {
                crate::advanced_workflow_state::initialize_workflow_state_manager(
                    crate::advanced_workflow_state::WorkflowStateConfig::default()
                ).await?;
                log_info!("lazy_loader", "✅ Workflow state management loaded");
            },
            
            Component::PredictiveErrorPrevention => {
                let _prevention = crate::predictive_error_prevention::PredictiveErrorPrevention::new(
                    crate::predictive_error_prevention::PredictiveConfig::default()
                ).await?;
                log_info!("lazy_loader", "✅ Predictive error prevention loaded");
            },
            
            Component::ContextAwareExecution => {
                crate::context_aware_execution::initialize_context_aware_executor(
                    crate::context_aware_execution::ContextAwareConfig::default()
                ).await?;
                log_info!("lazy_loader", "✅ Context-aware execution loaded");
            },
            
            Component::TestInfrastructure => {
                let _test_infra = crate::test_infrastructure::TestInfrastructure::new(
                    crate::test_infrastructure::TestInfrastructureConfig::default()
                ).await?;
                log_info!("lazy_loader", "✅ Test infrastructure loaded");
            },
            
            Component::EdgeCaseMastery => {
                crate::edge_case_mastery::initialize_edge_case_mastery(
                    crate::edge_case_mastery::EdgeCaseMasteryConfig::default()
                ).await?;
                log_info!("lazy_loader", "✅ Edge case mastery loaded");
            },
            
            Component::McpServers => {
                crate::mcp_manager::initialize_mcp().await?;
                log_info!("lazy_loader", "✅ MCP servers loaded");
            },
            
            Component::PromptManager => {
                // PromptManager is created per-command, so this is more of a validation
                log_info!("lazy_loader", "✅ Prompt manager ready");
            },
            
            Component::OpenRouterClient => {
                // OpenRouterClient is created per-command, so this is more of a validation
                log_info!("lazy_loader", "✅ OpenRouter client ready");
            },
            
            Component::WorkflowOrchestrator => {
                // WorkflowOrchestrator will be created when needed
                log_info!("lazy_loader", "✅ Workflow orchestrator ready");
            },
            
            Component::TaskExecutor => {
                // TaskExecutor will be created when needed
                log_info!("lazy_loader", "✅ Task executor ready");
            },
            
            Component::ChatInterface => {
                // ChatInterface will be created when needed
                log_info!("lazy_loader", "✅ Chat interface ready");
            },
        }
        
        loaded.insert(component.clone());
        Ok(())
    }

    /// Check if a component is loaded
    pub async fn is_loaded(&self, component: &Component) -> bool {
        let loaded = self.loaded_components.lock().await;
        loaded.contains(component)
    }

    /// Get all loaded components
    pub async fn get_loaded_components(&self) -> Vec<Component> {
        let loaded = self.loaded_components.lock().await;
        loaded.iter().cloned().collect()
    }

    /// Unload a component (for testing or cleanup)
    pub async fn unload_component(&self, component: &Component) {
        let mut loaded = self.loaded_components.lock().await;
        loaded.remove(component);
        log_debug!("lazy_loader", "Component {:?} unloaded", component);
    }

    /// Clear all loaded components
    pub async fn clear_all(&self) {
        let mut loaded = self.loaded_components.lock().await;
        loaded.clear();
        log_debug!("lazy_loader", "All components unloaded");
    }
}

/// Global lazy loader instance
static GLOBAL_LAZY_LOADER: Lazy<LazyLoader> = Lazy::new(|| LazyLoader::new());

/// Get the global lazy loader instance
pub fn get_lazy_loader() -> &'static LazyLoader {
    &GLOBAL_LAZY_LOADER
}

/// Initialize lazy loading for a command
pub async fn initialize_for_command(command: &str) -> Result<()> {
    get_lazy_loader().load_for_command(command).await
}

/// Load a specific component globally
pub async fn load_component(component: Component) -> Result<()> {
    get_lazy_loader().load_component(&component).await
}

/// Check if a component is loaded globally
pub async fn is_component_loaded(component: &Component) -> bool {
    get_lazy_loader().is_loaded(component).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lazy_loader_creation() {
        let loader = LazyLoader::new();
        assert!(!loader.is_loaded(&Component::ErrorRecovery).await);
    }

    #[tokio::test]
    async fn test_command_requirements() {
        let requirements = LazyLoader::get_command_requirements("chat");
        assert_eq!(requirements.command, "chat");
        assert!(requirements.required_components.contains(&Component::PromptManager));
        assert!(requirements.required_components.contains(&Component::ChatInterface));
    }

    #[tokio::test]
    async fn test_component_loading_tracking() {
        let loader = LazyLoader::new();
        
        // Initially nothing is loaded
        assert!(!loader.is_loaded(&Component::PromptManager).await);
        
        // Load a component
        let _ = loader.load_component(&Component::PromptManager).await;
        assert!(loader.is_loaded(&Component::PromptManager).await);
        
        // Unload the component
        loader.unload_component(&Component::PromptManager).await;
        assert!(!loader.is_loaded(&Component::PromptManager).await);
    }

    #[tokio::test]
    async fn test_global_lazy_loader() {
        // Test that global instance works
        let loader = get_lazy_loader();
        
        // Clear any previous state
        loader.clear_all().await;
        
        // Test component loading
        assert!(!is_component_loaded(&Component::PromptManager).await);
        let _ = load_component(Component::PromptManager).await;
        assert!(is_component_loaded(&Component::PromptManager).await);
    }
}