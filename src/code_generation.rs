use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::time::{timeout, Duration};

use crate::logger::{log_debug, log_info, log_warn};
use crate::openrouter_client::{OpenRouterClient, ChatMessage};
use crate::file_operations::{get_file_operations_manager, WriteFileParams, EditParams};
use crate::shell_execution::{get_shell_execution_manager, ShellParams};

/// Code generation context for tracking variables and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenContext {
    /// Variables available in the current context
    pub variables: HashMap<String, String>,
    /// Project root directory
    pub project_root: PathBuf,
    /// Current working directory
    pub working_dir: PathBuf,
    /// Project type (react, node, rust, etc.)
    pub project_type: Option<String>,
    /// Framework being used (if any)
    pub framework: Option<String>,
}

impl CodeGenContext {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            variables: HashMap::new(),
            project_root: project_root.clone(),
            working_dir: project_root,
            project_type: None,
            framework: None,
        }
    }

    pub fn set_variable(&mut self, key: &str, value: &str) {
        self.variables.insert(key.to_string(), value.to_string());
    }

    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    /// Template string replacement with context variables
    pub fn template_string(&self, template: &str) -> Result<String> {
        let mut result = template.to_string();
        
        for (key, value) in &self.variables {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        // Check for unresolved placeholders
        if result.contains("${") {
            log_warn!("code_gen", "⚠️ Template contains unresolved placeholders: {}", result);
        }
        
        Ok(result)
    }
}

/// Configuration for code generation tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenConfig {
    /// Model to use for generation
    pub model: String,
    /// Temperature for creativity
    pub temperature: f32,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Timeout for generation in seconds
    pub timeout_seconds: u64,
}

impl Default for CodeGenConfig {
    fn default() -> Self {
        Self {
            model: "gemini-2.5-pro".to_string(),
            temperature: 0.7,
            max_tokens: Some(4000),
            timeout_seconds: 60,
        }
    }
}

/// Parameters for code generation requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenRequest {
    /// The task description or prompt
    pub prompt: String,
    /// File path to generate (if creating a new file)
    pub target_file: Option<PathBuf>,
    /// Context for the generation
    pub context: CodeGenContext,
    /// Configuration for generation
    pub config: CodeGenConfig,
    /// Whether to execute shell commands if suggested
    pub allow_shell_execution: bool,
}

/// Result of code generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenResult {
    /// Generated code content
    pub generated_code: String,
    /// File that was created/modified
    pub target_file: Option<PathBuf>,
    /// Shell commands that were executed
    pub executed_commands: Vec<String>,
    /// Any variables emitted during generation
    pub emitted_variables: HashMap<String, String>,
    /// Execution time in seconds
    pub execution_time: f64,
}

/// Termination reason for code generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeGenTerminationReason {
    /// Successfully completed the task
    Success,
    /// Hit timeout limit
    Timeout,
    /// Encountered an error
    Error(String),
    /// Maximum iterations reached
    MaxIterations,
}

/// Core code generation engine inspired by gemini-cli's contentGenerator and subagent
pub struct CodeGenerationEngine {
    openrouter_client: OpenRouterClient,
}

impl CodeGenerationEngine {
    pub async fn new() -> Result<Self> {
        let openrouter_client = OpenRouterClient::new().await?;
        Ok(Self {
            openrouter_client,
        })
    }

    /// Generate code based on a request with context
    pub async fn generate_code(&self, request: CodeGenRequest) -> Result<CodeGenResult> {
        log_info!("code_gen", "🎯 Starting code generation: {}", 
                 request.prompt.chars().take(80).collect::<String>());

        let start_time = std::time::Instant::now();

        // Create system prompt for code generation
        let system_prompt = self.create_system_prompt(&request)?;
        
        // Build the user prompt with context
        let user_prompt = self.build_user_prompt(&request)?;

        // Execute the generation with timeout
        let timeout_duration = Duration::from_secs(request.config.timeout_seconds);
        let generation_result = timeout(
            timeout_duration,
            self.execute_generation(system_prompt, user_prompt, &request)
        ).await
        .context("Code generation timed out")?
        .context("Code generation failed")?;

        let execution_time = start_time.elapsed().as_secs_f64();

        log_info!("code_gen", "✅ Code generation completed in {:.2}s", execution_time);
        
        Ok(CodeGenResult {
            generated_code: generation_result.code,
            target_file: generation_result.target_file,
            executed_commands: generation_result.executed_commands,
            emitted_variables: generation_result.emitted_variables,
            execution_time,
        })
    }

    /// Generate React component with TypeScript
    pub async fn generate_react_component(&self, 
        component_name: &str, 
        description: &str,
        props: &[(&str, &str)],
        project_root: PathBuf
    ) -> Result<CodeGenResult> {
        let mut context = CodeGenContext::new(project_root);
        context.set_variable("component_name", component_name);
        context.set_variable("description", description);
        context.project_type = Some("react".to_string());
        context.framework = Some("typescript".to_string());

        // Build props interface
        let props_interface = if !props.is_empty() {
            let props_str = props.iter()
                .map(|(name, type_)| format!("  {}: {};", name, type_))
                .collect::<Vec<_>>()
                .join("\n");
            format!("interface {}Props {{\n{}\n}}", component_name, props_str)
        } else {
            format!("interface {}Props {{}}", component_name)
        };
        context.set_variable("props_interface", &props_interface);

        let target_file = context.project_root
            .join("src")
            .join("components")
            .join(format!("{}.tsx", component_name));

        let request = CodeGenRequest {
            prompt: format!(
                "Create a React TypeScript component named '{}' that {}. \
                Use the provided props interface. Follow React best practices and include proper TypeScript types.",
                component_name, description
            ),
            target_file: Some(target_file),
            context,
            config: CodeGenConfig::default(),
            allow_shell_execution: false,
        };

        self.generate_code(request).await
    }

    /// Generate TypeScript interface/type definitions
    pub async fn generate_typescript_types(&self,
        type_name: &str,
        description: &str,
        fields: &[(&str, &str)],
        project_root: PathBuf
    ) -> Result<CodeGenResult> {
        let mut context = CodeGenContext::new(project_root);
        context.set_variable("type_name", type_name);
        context.set_variable("description", description);
        context.project_type = Some("typescript".to_string());

        let target_file = context.project_root
            .join("src")
            .join("types")
            .join(format!("{}.ts", type_name.to_lowercase()));

        let fields_str = fields.iter()
            .map(|(name, type_)| format!("  {}: {};", name, type_))
            .collect::<Vec<_>>()
            .join("\n");
        context.set_variable("fields", &fields_str);

        let request = CodeGenRequest {
            prompt: format!(
                "Create TypeScript type definitions for '{}' that {}. \
                Include proper JSDoc comments and export the types for use in other modules.",
                type_name, description
            ),
            target_file: Some(target_file),
            context,
            config: CodeGenConfig::default(),
            allow_shell_execution: false,
        };

        self.generate_code(request).await
    }

    /// Generate test files for existing code
    pub async fn generate_test_file(&self,
        source_file: &PathBuf,
        test_type: &str, // "unit", "integration", "e2e"
        project_root: PathBuf
    ) -> Result<CodeGenResult> {
        let mut context = CodeGenContext::new(project_root);
        context.set_variable("source_file", &source_file.to_string_lossy());
        context.set_variable("test_type", test_type);

        // Read the source file to understand what to test
        let file_ops = get_file_operations_manager();
        let source_content = file_ops.read_file(source_file).await
            .context("Failed to read source file for test generation")?;
        context.set_variable("source_content", &source_content);

        // Determine test file path based on project structure
        let test_file = if source_file.extension().map_or(false, |ext| ext == "tsx" || ext == "ts") {
            // TypeScript/React project
            let stem = source_file.file_stem().unwrap().to_string_lossy();
            source_file.parent().unwrap().join(format!("{}.test.ts", stem))
        } else {
            // Fallback
            let stem = source_file.file_stem().unwrap().to_string_lossy();
            source_file.parent().unwrap().join(format!("{}.test", stem))
        };

        let request = CodeGenRequest {
            prompt: format!(
                "Generate comprehensive {} tests for the code in {}. \
                Include test cases for all public functions/methods, edge cases, and error conditions. \
                Use appropriate testing frameworks and follow testing best practices.",
                test_type, source_file.display()
            ),
            target_file: Some(test_file),
            context,
            config: CodeGenConfig::default(),
            allow_shell_execution: false,
        };

        self.generate_code(request).await
    }

    /// Create system prompt for code generation
    fn create_system_prompt(&self, request: &CodeGenRequest) -> Result<String> {
        let base_prompt = r#"
You are a specialized code generation agent designed to create high-quality, production-ready code.

# Core Responsibilities
- Generate clean, well-structured, and maintainable code
- Follow established coding conventions and best practices
- Include proper error handling and type safety
- Add meaningful comments where necessary
- Ensure code is ready for production use

# Code Quality Standards
- Use consistent naming conventions
- Implement proper error handling
- Include type annotations (TypeScript, Rust, etc.)
- Follow security best practices
- Write self-documenting code with clear intent

# Project Context Awareness
- Adapt to the existing project structure and conventions
- Use established patterns and frameworks
- Respect existing dependencies and tooling
- Maintain consistency with the codebase style

# Output Format
Generate only the requested code without additional explanation unless specifically asked.
Focus on correctness, readability, and maintainability.
"#;

        // Customize based on project type and context
        let project_specific = match request.context.project_type.as_deref() {
            Some("react") => "\n# React/TypeScript Specific Guidelines\n\
                - Use functional components with hooks\n\
                - Implement proper TypeScript interfaces\n\
                - Follow React best practices for state management\n\
                - Use modern React patterns (Context, custom hooks)\n\
                - Ensure accessibility compliance",
            Some("rust") => "\n# Rust Specific Guidelines\n\
                - Use idiomatic Rust patterns\n\
                - Implement proper error handling with Result<T, E>\n\
                - Follow Rust naming conventions\n\
                - Use appropriate ownership patterns\n\
                - Include comprehensive documentation",
            Some("node") => "\n# Node.js Specific Guidelines\n\
                - Use modern JavaScript/TypeScript features\n\
                - Implement proper async/await patterns\n\
                - Include appropriate error handling\n\
                - Follow Node.js best practices\n\
                - Use environment configuration properly",
            _ => "",
        };

        Ok(format!("{}{}", base_prompt, project_specific))
    }

    /// Build user prompt with context variables
    fn build_user_prompt(&self, request: &CodeGenRequest) -> Result<String> {
        let templated_prompt = request.context.template_string(&request.prompt)?;
        
        let mut prompt_parts = vec![templated_prompt];

        // Add context information
        if !request.context.variables.is_empty() {
            let context_vars = request.context.variables.iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join("\n");
            prompt_parts.push(format!("\n## Context Variables\n{}", context_vars));
        }

        if let Some(project_type) = &request.context.project_type {
            prompt_parts.push(format!("\n## Project Type\n{}", project_type));
        }

        if let Some(framework) = &request.context.framework {
            prompt_parts.push(format!("\n## Framework\n{}", framework));
        }

        Ok(prompt_parts.join("\n"))
    }

    /// Execute the actual generation process
    async fn execute_generation(&self, 
        system_prompt: String, 
        user_prompt: String, 
        request: &CodeGenRequest
    ) -> Result<GenerationResult> {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];

        // Call OpenRouter API for code generation
        let generated_code = self.openrouter_client.chat_completion(messages).await
            .context("Failed to get code generation response")?;

        let mut result = GenerationResult {
            code: generated_code,
            target_file: None,
            executed_commands: Vec::new(),
            emitted_variables: HashMap::new(),
        };

        // Write to file if target specified
        if let Some(target_file) = &request.target_file {
            let file_ops = get_file_operations_manager();
            
            // Ensure parent directory exists
            if let Some(parent) = target_file.parent() {
                if !parent.exists() {
                    let shell_manager = get_shell_execution_manager();
                    let mkdir_cmd = ShellParams {
                        command: format!("mkdir -p \"{}\"", parent.display()),
                        description: Some("Create directory structure".to_string()),
                        directory: Some(request.context.project_root.clone()),
                        timeout_seconds: Some(30),
                    };
                    
                    match shell_manager.execute_command(mkdir_cmd).await {
                        Ok(shell_result) => {
                            if shell_result.success {
                                result.executed_commands.push(shell_result.command);
                                log_debug!("code_gen", "📁 Created directory: {}", parent.display());
                            }
                        }
                        Err(e) => log_warn!("code_gen", "⚠️ Failed to create directory: {}", e),
                    }
                }
            }

            // Write the generated code to file
            let write_params = WriteFileParams {
                file_path: target_file.clone(),
                content: result.code.clone(),
            };

            match file_ops.write_file(write_params).await {
                Ok(_) => {
                    result.target_file = Some(target_file.clone());
                    log_info!("code_gen", "📝 Generated code written to: {}", target_file.display());
                }
                Err(e) => {
                    log_warn!("code_gen", "⚠️ Failed to write generated code: {}", e);
                }
            }
        }

        Ok(result)
    }
}

/// Internal result structure for generation
#[derive(Debug)]
struct GenerationResult {
    code: String,
    target_file: Option<PathBuf>,
    executed_commands: Vec<String>,
    emitted_variables: HashMap<String, String>,
}

/// Template definitions for common code patterns
pub struct CodeTemplates;

impl CodeTemplates {
    /// Get React component template
    pub fn react_component_template() -> &'static str {
        r#"import React from 'react';

${props_interface}

const ${component_name}: React.FC<${component_name}Props> = (props) => {
  return (
    <div className="${component_name.toLowerCase()}">
      {/* ${description} */}
      <h2>${component_name}</h2>
    </div>
  );
};

export default ${component_name};
"#
    }

    /// Get TypeScript interface template
    pub fn typescript_interface_template() -> &'static str {
        r#"/**
 * ${description}
 */
export interface ${type_name} {
${fields}
}

/**
 * Type guard for ${type_name}
 */
export function is${type_name}(obj: any): obj is ${type_name} {
  return obj && typeof obj === 'object';
}
"#
    }

    /// Get test file template
    pub fn test_file_template() -> &'static str {
        r#"import { describe, it, expect } from '@jest/globals';

describe('${test_subject}', () => {
  it('should ${test_description}', () => {
    // Test implementation
    expect(true).toBe(true);
  });

  it('should handle edge cases', () => {
    // Edge case testing
    expect(true).toBe(true);
  });

  it('should handle errors gracefully', () => {
    // Error handling testing
    expect(true).toBe(true);
  });
});
"#
    }
}

/// Get code generation engine instance (creates new instance each time)
pub async fn get_code_generation_engine() -> Result<CodeGenerationEngine> {
    CodeGenerationEngine::new().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_context_template_string() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = CodeGenContext::new(temp_dir.path().to_path_buf());
        context.set_variable("name", "TestComponent");
        context.set_variable("description", "A test component");

        let template = "Create component ${name} that ${description}";
        let result = context.template_string(template).unwrap();
        
        assert_eq!(result, "Create component TestComponent that A test component");
    }

    #[test]
    fn test_react_component_template() {
        let template = CodeTemplates::react_component_template();
        assert!(template.contains("${component_name}"));
        assert!(template.contains("${props_interface}"));
        assert!(template.contains("${description}"));
    }
}