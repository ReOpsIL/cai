//! Enhanced tools for CAI - addressing the 57% failure rate in complex scenarios
//! 
//! This module implements sophisticated multi-file project generation, automated testing,
//! documentation generation, and quality validation based on patterns from crush and gemini-cli.

pub mod project_generator;
pub mod test_generator;
pub mod doc_generator;
pub mod quality_validator;
pub mod multi_file_safety;
pub mod templates;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a complete project structure with multiple files and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStructure {
    pub name: String,
    pub language: String,
    pub framework: Option<String>,
    pub files: HashMap<PathBuf, String>,
    pub directories: Vec<PathBuf>,
    pub metadata: ProjectMetadata,
}

/// Enhanced metadata for project generation quality assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub complexity_factors: Vec<String>,
    pub estimated_quality_score: f32,
    pub has_tests: bool,
    pub has_documentation: bool,
    pub has_configuration: bool,
    pub security_considerations: Vec<String>,
    pub performance_notes: Vec<String>,
    pub deployment_instructions: Option<String>,
}

/// Project template with language-specific patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTemplate {
    pub name: String,
    pub language: String,
    pub framework: Option<String>,
    pub file_templates: HashMap<String, FileTemplate>,
    pub directory_structure: Vec<String>,
    pub dependencies: Vec<String>,
    pub quality_requirements: QualityRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTemplate {
    pub path: String,
    pub content_template: String,
    pub is_executable: bool,
    pub variables: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityRequirements {
    pub min_quality_score: f32,
    pub required_files: Vec<String>,
    pub required_tests: bool,
    pub required_documentation: bool,
    pub security_checks: Vec<String>,
}

/// Multi-file operation for atomic project creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiFileOperation {
    pub file_path: PathBuf,
    pub content: String,
    pub operation_type: FileOperationType,
    pub permissions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOperationType {
    Create,
    Update,
    Delete,
    Mkdir,
}

/// Quality gates that must pass before project creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGate {
    pub name: String,
    pub description: String,
    pub validator: QualityValidator,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityValidator {
    SyntaxCheck,
    SecurityScan,
    TestCoverage,
    DocumentationPresence,
    CodeComplexity,
    PerformanceCheck,
}

/// Enhanced tool response with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedToolResponse {
    pub success: bool,
    pub message: String,
    pub files_created: Vec<PathBuf>,
    pub quality_score: f32,
    pub quality_gates_passed: Vec<String>,
    pub quality_gates_failed: Vec<String>,
    pub suggestions: Vec<String>,
    pub execution_time_ms: u64,
    pub project_structure: Option<ProjectStructure>,
}

impl EnhancedToolResponse {
    pub fn success(message: String, project: ProjectStructure) -> Self {
        Self {
            success: true,
            message,
            files_created: project.files.keys().cloned().collect(),
            quality_score: project.metadata.estimated_quality_score,
            quality_gates_passed: vec![],
            quality_gates_failed: vec![],
            suggestions: vec![],
            execution_time_ms: 0,
            project_structure: Some(project),
        }
    }

    pub fn failure(message: String, suggestions: Vec<String>) -> Self {
        Self {
            success: false,
            message,
            files_created: vec![],
            quality_score: 0.0,
            quality_gates_passed: vec![],
            quality_gates_failed: vec![],
            suggestions,
            execution_time_ms: 0,
            project_structure: None,
        }
    }
}

/// Error types for enhanced tools
#[derive(Debug)]
pub enum EnhancedToolError {
    QualityGateFailed { gate: String, reason: String },
    MultiFileOperationFailed { operation: String, reason: String },
    TemplateNotFound { template: String },
    PermissionDenied { operation: String },
    Io(std::io::Error),
    Serialization(serde_json::Error),
}

impl std::fmt::Display for EnhancedToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnhancedToolError::QualityGateFailed { gate, reason } => 
                write!(f, "Quality gate failed: {} - {}", gate, reason),
            EnhancedToolError::MultiFileOperationFailed { operation, reason } => 
                write!(f, "Multi-file operation failed: {} - {}", operation, reason),
            EnhancedToolError::TemplateNotFound { template } => 
                write!(f, "Template not found: {}", template),
            EnhancedToolError::PermissionDenied { operation } => 
                write!(f, "Permission denied for operation: {}", operation),
            EnhancedToolError::Io(e) => write!(f, "IO error: {}", e),
            EnhancedToolError::Serialization(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl std::error::Error for EnhancedToolError {}

impl From<std::io::Error> for EnhancedToolError {
    fn from(e: std::io::Error) -> Self {
        EnhancedToolError::Io(e)
    }
}

impl From<serde_json::Error> for EnhancedToolError {
    fn from(e: serde_json::Error) -> Self {
        EnhancedToolError::Serialization(e)
    }
}

// Re-export main components for easy access
pub use project_generator::ProjectGenerator;
pub use test_generator::TestGenerator;
pub use doc_generator::DocumentationGenerator;
pub use quality_validator::QualityValidator as QualityValidatorTool;
pub use multi_file_safety::MultiFileSafetyValidator;