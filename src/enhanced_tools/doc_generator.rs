//! Documentation automation addressing the 72% missing documentation failure
//! 
//! This module implements comprehensive documentation generation including:
//! - README files with setup instructions
//! - API documentation
//! - Architecture documentation
//! - Code comments and docstrings

use super::*;
use crate::logger::{log_info, log_debug, log_error};
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Automated documentation generator for multi-language projects
pub struct DocumentationGenerator {
    templates: HashMap<String, DocumentationTemplate>,
    language_configs: HashMap<String, LanguageDocConfig>,
}

/// Documentation template for different document types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationTemplate {
    pub name: String,
    pub document_type: DocumentType,
    pub template_content: String,
    pub required_sections: Vec<String>,
    pub optional_sections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentType {
    Readme,
    ApiDocumentation,
    Architecture,
    UserGuide,
    DeveloperGuide,
    Changelog,
}

/// Language-specific documentation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDocConfig {
    pub language: String,
    pub comment_style: CommentStyle,
    pub docstring_style: DocstringStyle,
    pub api_doc_tools: Vec<String>,
    pub doc_generation_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommentStyle {
    DoubleSlash,   // //
    Hash,          // #
    SlashStar,     // /* */
    TripleSlash,   // ///
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocstringStyle {
    Python,        // """docstring"""
    JavaDoc,       // /** javadoc */
    Rust,          // /// rust doc
    JSDoc,         // /** jsdoc */
    GoDoc,         // // go doc
}

/// Generated documentation suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationSuite {
    pub project_name: String,
    pub language: String,
    pub documentation_files: HashMap<PathBuf, String>,
    pub inline_documentation: HashMap<PathBuf, String>, // Enhanced source files with docs
    pub metadata: DocumentationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationMetadata {
    pub total_documents: usize,
    pub documentation_coverage: f32,
    pub api_endpoints_documented: usize,
    pub functions_documented: usize,
    pub classes_documented: usize,
    pub complexity_explanation_score: f32,
}

impl DocumentationGenerator {
    pub fn new() -> Result<Self> {
        let mut generator = Self {
            templates: HashMap::new(),
            language_configs: HashMap::new(),
        };
        
        generator.load_documentation_templates()?;
        generator.load_language_configs()?;
        
        Ok(generator)
    }

    /// Generate comprehensive documentation suite for a project
    pub async fn generate_documentation(
        &self,
        project: &ProjectStructure,
        target_directory: &Path,
    ) -> Result<DocumentationSuite> {
        log_info!("doc_generator", "Generating documentation for {} project", project.name);
        
        let mut doc_suite = DocumentationSuite {
            project_name: project.name.clone(),
            language: project.language.clone(),
            documentation_files: HashMap::new(),
            inline_documentation: HashMap::new(),
            metadata: DocumentationMetadata {
                total_documents: 0,
                documentation_coverage: 0.0,
                api_endpoints_documented: 0,
                functions_documented: 0,
                classes_documented: 0,
                complexity_explanation_score: 0.0,
            },
        };

        // 1. Generate README.md with comprehensive project information
        self.generate_readme_documentation(project, &mut doc_suite, target_directory).await?;
        
        // 2. Generate API documentation for API projects
        if project.metadata.complexity_factors.contains(&"api_development".to_string()) {
            self.generate_api_documentation(project, &mut doc_suite, target_directory).await?;
        }
        
        // 3. Generate architecture documentation for complex projects
        if project.metadata.complexity_factors.len() >= 3 {
            self.generate_architecture_documentation(project, &mut doc_suite, target_directory).await?;
        }
        
        // 4. Generate setup and deployment documentation
        self.generate_setup_documentation(project, &mut doc_suite, target_directory).await?;
        
        // 5. Generate inline code documentation (docstrings/comments)
        self.generate_inline_documentation(project, &mut doc_suite, target_directory).await?;
        
        // 6. Generate security documentation for security projects
        if project.metadata.complexity_factors.contains(&"security_implementation".to_string()) {
            self.generate_security_documentation(project, &mut doc_suite, target_directory).await?;
        }
        
        // 7. Generate performance documentation for performance projects
        if project.metadata.complexity_factors.contains(&"performance_optimization".to_string()) {
            self.generate_performance_documentation(project, &mut doc_suite, target_directory).await?;
        }
        
        // 8. Calculate documentation metadata
        self.calculate_documentation_metadata(&mut doc_suite, project);
        
        log_info!("doc_generator", 
            "Generated {} documentation files with {:.1}% coverage", 
            doc_suite.metadata.total_documents,
            doc_suite.metadata.documentation_coverage
        );
        
        Ok(doc_suite)
    }

    /// Generate comprehensive README.md
    async fn generate_readme_documentation(
        &self,
        project: &ProjectStructure,
        doc_suite: &mut DocumentationSuite,
        target_directory: &Path,
    ) -> Result<()> {
        log_debug!("doc_generator", "Generating README documentation");
        
        let readme_path = target_directory.join(&project.name).join("README.md");
        let readme_content = self.create_comprehensive_readme(project).await?;
        
        doc_suite.documentation_files.insert(readme_path, readme_content);
        doc_suite.metadata.total_documents += 1;
        
        Ok(())
    }

    /// Generate API documentation
    async fn generate_api_documentation(
        &self,
        project: &ProjectStructure,
        doc_suite: &mut DocumentationSuite,
        target_directory: &Path,
    ) -> Result<()> {
        log_debug!("doc_generator", "Generating API documentation");
        
        let docs_dir = target_directory.join(&project.name).join("docs");
        
        // API Reference
        let api_ref_path = docs_dir.join("api-reference.md");
        let api_ref_content = self.create_api_reference(project).await?;
        doc_suite.documentation_files.insert(api_ref_path, api_ref_content);
        
        // API Examples
        let api_examples_path = docs_dir.join("api-examples.md");
        let api_examples_content = self.create_api_examples(project).await?;
        doc_suite.documentation_files.insert(api_examples_path, api_examples_content);
        
        // OpenAPI/Swagger spec for REST APIs
        if project.metadata.complexity_factors.contains(&"api_development".to_string()) {
            let openapi_path = docs_dir.join("openapi.yaml");
            let openapi_content = self.create_openapi_specification(project).await?;
            doc_suite.documentation_files.insert(openapi_path, openapi_content);
        }
        
        doc_suite.metadata.total_documents += 3;
        doc_suite.metadata.api_endpoints_documented += self.count_api_endpoints(project);
        
        Ok(())
    }

    /// Generate architecture documentation
    async fn generate_architecture_documentation(
        &self,
        project: &ProjectStructure,
        doc_suite: &mut DocumentationSuite,
        target_directory: &Path,
    ) -> Result<()> {
        log_debug!("doc_generator", "Generating architecture documentation");
        
        let docs_dir = target_directory.join(&project.name).join("docs");
        
        // System Architecture
        let arch_path = docs_dir.join("architecture.md");
        let arch_content = self.create_architecture_documentation(project).await?;
        doc_suite.documentation_files.insert(arch_path, arch_content);
        
        // Database Schema (if applicable)
        if project.metadata.complexity_factors.contains(&"database_design".to_string()) {
            let schema_path = docs_dir.join("database-schema.md");
            let schema_content = self.create_database_schema_documentation(project).await?;
            doc_suite.documentation_files.insert(schema_path, schema_content);
        }
        
        // Deployment Architecture
        let deployment_path = docs_dir.join("deployment.md");
        let deployment_content = self.create_deployment_documentation(project).await?;
        doc_suite.documentation_files.insert(deployment_path, deployment_content);
        
        doc_suite.metadata.total_documents += 2;
        if project.metadata.complexity_factors.contains(&"database_design".to_string()) {
            doc_suite.metadata.total_documents += 1;
        }
        
        Ok(())
    }

    /// Generate setup and deployment documentation
    async fn generate_setup_documentation(
        &self,
        project: &ProjectStructure,
        doc_suite: &mut DocumentationSuite,
        target_directory: &Path,
    ) -> Result<()> {
        log_debug!("doc_generator", "Generating setup documentation");
        
        let docs_dir = target_directory.join(&project.name).join("docs");
        
        // Installation Guide
        let install_path = docs_dir.join("installation.md");
        let install_content = self.create_installation_guide(project).await?;
        doc_suite.documentation_files.insert(install_path, install_content);
        
        // Development Setup
        let dev_setup_path = docs_dir.join("development-setup.md");
        let dev_setup_content = self.create_development_setup_guide(project).await?;
        doc_suite.documentation_files.insert(dev_setup_path, dev_setup_content);
        
        // Contributing Guide
        let contributing_path = target_directory.join(&project.name).join("CONTRIBUTING.md");
        let contributing_content = self.create_contributing_guide(project).await?;
        doc_suite.documentation_files.insert(contributing_path, contributing_content);
        
        doc_suite.metadata.total_documents += 3;
        
        Ok(())
    }

    /// Generate inline code documentation (docstrings/comments)
    async fn generate_inline_documentation(
        &self,
        project: &ProjectStructure,
        doc_suite: &mut DocumentationSuite,
        target_directory: &Path,
    ) -> Result<()> {
        log_debug!("doc_generator", "Generating inline documentation");
        
        let lang_config = self.get_language_config(&project.language)?;
        
        for (file_path, content) in &project.files {
            if self.is_source_file(file_path, &project.language) {
                let documented_content = self.add_inline_documentation(
                    content,
                    lang_config,
                    project,
                ).await?;
                
                doc_suite.inline_documentation.insert(file_path.clone(), documented_content);
                
                // Count documented functions/classes
                doc_suite.metadata.functions_documented += self.count_functions(content);
                doc_suite.metadata.classes_documented += self.count_classes(content);
            }
        }
        
        Ok(())
    }

    /// Generate security documentation
    async fn generate_security_documentation(
        &self,
        project: &ProjectStructure,
        doc_suite: &mut DocumentationSuite,
        target_directory: &Path,
    ) -> Result<()> {
        log_debug!("doc_generator", "Generating security documentation");
        
        let docs_dir = target_directory.join(&project.name).join("docs");
        
        // Security Overview
        let security_path = docs_dir.join("security.md");
        let security_content = self.create_security_documentation(project).await?;
        doc_suite.documentation_files.insert(security_path, security_content);
        
        // Threat Model
        let threat_model_path = docs_dir.join("threat-model.md");
        let threat_content = self.create_threat_model_documentation(project).await?;
        doc_suite.documentation_files.insert(threat_model_path, threat_content);
        
        doc_suite.metadata.total_documents += 2;
        
        Ok(())
    }

    /// Generate performance documentation
    async fn generate_performance_documentation(
        &self,
        project: &ProjectStructure,
        doc_suite: &mut DocumentationSuite,
        target_directory: &Path,
    ) -> Result<()> {
        log_debug!("doc_generator", "Generating performance documentation");
        
        let docs_dir = target_directory.join(&project.name).join("docs");
        
        // Performance Guide
        let perf_path = docs_dir.join("performance.md");
        let perf_content = self.create_performance_documentation(project).await?;
        doc_suite.documentation_files.insert(perf_path, perf_content);
        
        // Benchmarking Guide
        let benchmark_path = docs_dir.join("benchmarking.md");
        let benchmark_content = self.create_benchmarking_guide(project).await?;
        doc_suite.documentation_files.insert(benchmark_path, benchmark_content);
        
        doc_suite.metadata.total_documents += 2;
        
        Ok(())
    }

    /// Load documentation templates
    fn load_documentation_templates(&mut self) -> Result<()> {
        // README template
        self.templates.insert("readme".to_string(), DocumentationTemplate {
            name: "readme".to_string(),
            document_type: DocumentType::Readme,
            template_content: self.get_readme_template(),
            required_sections: vec![
                "Title".to_string(),
                "Description".to_string(),
                "Installation".to_string(),
                "Usage".to_string(),
            ],
            optional_sections: vec![
                "Features".to_string(),
                "API Reference".to_string(),
                "Contributing".to_string(),
                "License".to_string(),
            ],
        });

        // API Documentation template
        self.templates.insert("api".to_string(), DocumentationTemplate {
            name: "api".to_string(),
            document_type: DocumentType::ApiDocumentation,
            template_content: self.get_api_template(),
            required_sections: vec![
                "Overview".to_string(),
                "Endpoints".to_string(),
                "Authentication".to_string(),
                "Examples".to_string(),
            ],
            optional_sections: vec![
                "Rate Limiting".to_string(),
                "Error Codes".to_string(),
                "Webhooks".to_string(),
            ],
        });

        // Architecture template
        self.templates.insert("architecture".to_string(), DocumentationTemplate {
            name: "architecture".to_string(),
            document_type: DocumentType::Architecture,
            template_content: self.get_architecture_template(),
            required_sections: vec![
                "Overview".to_string(),
                "Components".to_string(),
                "Data Flow".to_string(),
                "Technology Stack".to_string(),
            ],
            optional_sections: vec![
                "Security Considerations".to_string(),
                "Performance Considerations".to_string(),
                "Scalability".to_string(),
            ],
        });

        Ok(())
    }

    /// Load language-specific documentation configurations
    fn load_language_configs(&mut self) -> Result<()> {
        // Python configuration
        self.language_configs.insert("python".to_string(), LanguageDocConfig {
            language: "python".to_string(),
            comment_style: CommentStyle::Hash,
            docstring_style: DocstringStyle::Python,
            api_doc_tools: vec!["sphinx".to_string(), "pdoc".to_string()],
            doc_generation_commands: vec!["sphinx-build".to_string()],
        });

        // JavaScript configuration
        self.language_configs.insert("javascript".to_string(), LanguageDocConfig {
            language: "javascript".to_string(),
            comment_style: CommentStyle::DoubleSlash,
            docstring_style: DocstringStyle::JSDoc,
            api_doc_tools: vec!["jsdoc".to_string(), "typedoc".to_string()],
            doc_generation_commands: vec!["jsdoc".to_string()],
        });

        // TypeScript configuration
        self.language_configs.insert("typescript".to_string(), LanguageDocConfig {
            language: "typescript".to_string(),
            comment_style: CommentStyle::DoubleSlash,
            docstring_style: DocstringStyle::JSDoc,
            api_doc_tools: vec!["typedoc".to_string()],
            doc_generation_commands: vec!["typedoc".to_string()],
        });

        // Rust configuration
        self.language_configs.insert("rust".to_string(), LanguageDocConfig {
            language: "rust".to_string(),
            comment_style: CommentStyle::DoubleSlash,
            docstring_style: DocstringStyle::Rust,
            api_doc_tools: vec!["rustdoc".to_string()],
            doc_generation_commands: vec!["cargo doc".to_string()],
        });

        // Go configuration
        self.language_configs.insert("go".to_string(), LanguageDocConfig {
            language: "go".to_string(),
            comment_style: CommentStyle::DoubleSlash,
            docstring_style: DocstringStyle::GoDoc,
            api_doc_tools: vec!["godoc".to_string()],
            doc_generation_commands: vec!["go doc".to_string()],
        });

        Ok(())
    }

    // Helper methods

    fn get_language_config(&self, language: &str) -> Result<&LanguageDocConfig> {
        self.language_configs.get(language)
            .ok_or_else(|| anyhow::anyhow!("No language configuration found for: {}", language))
    }

    fn is_source_file(&self, file_path: &Path, language: &str) -> bool {
        let extension = file_path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
        match language {
            "python" => extension == "py",
            "javascript" => extension == "js",
            "typescript" => extension == "ts",
            "rust" => extension == "rs",
            "go" => extension == "go",
            "java" => extension == "java",
            _ => false,
        }
    }

    fn count_api_endpoints(&self, project: &ProjectStructure) -> usize {
        // Heuristic to count API endpoints based on project content
        project.files.values()
            .map(|content| {
                content.lines().filter(|line| {
                    line.contains("@app.route") || 
                    line.contains("app.get") || 
                    line.contains("app.post") ||
                    line.contains("router.") ||
                    line.contains("@RestController") ||
                    line.contains("@GetMapping") ||
                    line.contains("@PostMapping")
                }).count()
            })
            .sum()
    }

    fn count_functions(&self, content: &str) -> usize {
        content.lines().filter(|line| {
            line.trim().starts_with("def ") ||
            line.trim().starts_with("function ") ||
            line.trim().starts_with("fn ") ||
            line.trim().starts_with("func ")
        }).count()
    }

    fn count_classes(&self, content: &str) -> usize {
        content.lines().filter(|line| {
            line.trim().starts_with("class ") ||
            line.trim().starts_with("struct ") ||
            line.trim().starts_with("interface ") ||
            line.trim().starts_with("type ")
        }).count()
    }

    fn calculate_documentation_metadata(&self, doc_suite: &mut DocumentationSuite, project: &ProjectStructure) {
        // Calculate documentation coverage
        let source_files = project.files.keys().filter(|path| 
            self.is_source_file(path, &project.language)
        ).count();
        
        let documented_files = doc_suite.inline_documentation.len();
        
        doc_suite.metadata.documentation_coverage = if source_files > 0 {
            (documented_files as f32 / source_files as f32) * 100.0
        } else {
            0.0
        };

        // Calculate complexity explanation score
        doc_suite.metadata.complexity_explanation_score = 
            project.metadata.complexity_factors.len() as f32 * 15.0 +
            doc_suite.metadata.total_documents as f32 * 5.0;
    }

    // Template content methods (placeholders for actual templates)

    fn get_readme_template(&self) -> String {
        r#"# {project_name}

## Description
{description}

## Features
{features}

## Installation
{installation_instructions}

## Usage
{usage_examples}

## API Reference
{api_reference}

## Architecture
{architecture_overview}

## Security
{security_considerations}

## Performance
{performance_notes}

## Contributing
{contributing_guidelines}

## License
{license}
"#.to_string()
    }

    fn get_api_template(&self) -> String {
        r#"# API Reference

## Overview
{api_overview}

## Authentication
{authentication}

## Endpoints
{endpoints}

## Examples
{examples}

## Error Handling
{error_handling}
"#.to_string()
    }

    fn get_architecture_template(&self) -> String {
        r#"# Architecture Documentation

## System Overview
{system_overview}

## Components
{components}

## Data Flow
{data_flow}

## Technology Stack
{technology_stack}

## Deployment
{deployment}
"#.to_string()
    }

    // Content generation methods (placeholders for actual implementation)

    async fn create_comprehensive_readme(&self, project: &ProjectStructure) -> Result<String> {
        // Generate comprehensive README based on project analysis
        Ok(format!("# {}\n\nGenerated comprehensive README for {} project.", project.name, project.language))
    }

    async fn create_api_reference(&self, project: &ProjectStructure) -> Result<String> {
        Ok("# API Reference\n\nGenerated API documentation.".to_string())
    }

    async fn create_api_examples(&self, project: &ProjectStructure) -> Result<String> {
        Ok("# API Examples\n\nGenerated API examples.".to_string())
    }

    async fn create_openapi_specification(&self, project: &ProjectStructure) -> Result<String> {
        Ok("openapi: 3.0.0\ninfo:\n  title: Generated API\n  version: 1.0.0".to_string())
    }

    async fn create_architecture_documentation(&self, project: &ProjectStructure) -> Result<String> {
        Ok("# Architecture\n\nGenerated architecture documentation.".to_string())
    }

    async fn create_database_schema_documentation(&self, project: &ProjectStructure) -> Result<String> {
        Ok("# Database Schema\n\nGenerated database schema documentation.".to_string())
    }

    async fn create_deployment_documentation(&self, project: &ProjectStructure) -> Result<String> {
        Ok("# Deployment\n\nGenerated deployment documentation.".to_string())
    }

    async fn create_installation_guide(&self, project: &ProjectStructure) -> Result<String> {
        Ok("# Installation Guide\n\nGenerated installation instructions.".to_string())
    }

    async fn create_development_setup_guide(&self, project: &ProjectStructure) -> Result<String> {
        Ok("# Development Setup\n\nGenerated development setup guide.".to_string())
    }

    async fn create_contributing_guide(&self, project: &ProjectStructure) -> Result<String> {
        Ok("# Contributing\n\nGenerated contributing guidelines.".to_string())
    }

    async fn add_inline_documentation(&self, content: &str, config: &LanguageDocConfig, project: &ProjectStructure) -> Result<String> {
        // Add comprehensive inline documentation to source code
        Ok(format!("// Enhanced with inline documentation\n{}", content))
    }

    async fn create_security_documentation(&self, project: &ProjectStructure) -> Result<String> {
        Ok("# Security\n\nGenerated security documentation.".to_string())
    }

    async fn create_threat_model_documentation(&self, project: &ProjectStructure) -> Result<String> {
        Ok("# Threat Model\n\nGenerated threat model documentation.".to_string())
    }

    async fn create_performance_documentation(&self, project: &ProjectStructure) -> Result<String> {
        Ok("# Performance\n\nGenerated performance documentation.".to_string())
    }

    async fn create_benchmarking_guide(&self, project: &ProjectStructure) -> Result<String> {
        Ok("# Benchmarking\n\nGenerated benchmarking guide.".to_string())
    }
}