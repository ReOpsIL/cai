use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::logger::{log_debug, log_info, log_warn};
use crate::file_operations::{get_file_operations_manager, WriteFileParams};
use crate::shell_execution::{get_shell_execution_manager, ShellParams};
use crate::code_generation::{get_code_generation_engine, CodeGenContext};

/// Project template types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectTemplate {
    /// React TypeScript application
    ReactTypeScript,
    /// React TypeScript with Redux Toolkit
    ReactRedux,
    /// Node.js TypeScript API
    NodeTypeScript,
    /// Express.js API with TypeScript
    ExpressApi,
    /// Next.js full-stack application
    NextJs,
    /// Vue.js TypeScript application
    VueTypeScript,
    /// Vanilla TypeScript project
    TypeScriptLibrary,
}

impl ProjectTemplate {
    pub fn name(&self) -> &'static str {
        match self {
            ProjectTemplate::ReactTypeScript => "React TypeScript",
            ProjectTemplate::ReactRedux => "React Redux TypeScript",
            ProjectTemplate::NodeTypeScript => "Node.js TypeScript",
            ProjectTemplate::ExpressApi => "Express.js API",
            ProjectTemplate::NextJs => "Next.js Full-stack",
            ProjectTemplate::VueTypeScript => "Vue.js TypeScript",
            ProjectTemplate::TypeScriptLibrary => "TypeScript Library",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ProjectTemplate::ReactTypeScript => "Modern React application with TypeScript and Vite",
            ProjectTemplate::ReactRedux => "React with Redux Toolkit for state management",
            ProjectTemplate::NodeTypeScript => "Node.js backend with TypeScript",
            ProjectTemplate::ExpressApi => "RESTful API using Express.js and TypeScript",
            ProjectTemplate::NextJs => "Full-stack React application with Next.js",
            ProjectTemplate::VueTypeScript => "Vue.js application with TypeScript",
            ProjectTemplate::TypeScriptLibrary => "TypeScript library with build tooling",
        }
    }
}

/// Project scaffolding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaffoldConfig {
    /// Project name
    pub project_name: String,
    /// Project root directory
    pub project_root: PathBuf,
    /// Template to use
    pub template: ProjectTemplate,
    /// Additional features to include
    pub features: Vec<ProjectFeature>,
    /// Author information
    pub author: Option<String>,
    /// Description
    pub description: Option<String>,
    /// License
    pub license: Option<String>,
}

/// Additional features that can be included
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectFeature {
    /// Testing framework (Jest, Vitest, etc.)
    Testing,
    /// Linting with ESLint
    Linting,
    /// Code formatting with Prettier
    Formatting,
    /// Tailwind CSS for styling
    TailwindCss,
    /// Material-UI components
    MaterialUi,
    /// React Router for navigation
    ReactRouter,
    /// Styled Components
    StyledComponents,
    /// Storybook for component development
    Storybook,
    /// Docker configuration
    Docker,
    /// GitHub Actions CI/CD
    GithubActions,
    /// Husky git hooks
    GitHooks,
}

impl ProjectFeature {
    pub fn name(&self) -> &'static str {
        match self {
            ProjectFeature::Testing => "Testing Framework",
            ProjectFeature::Linting => "ESLint",
            ProjectFeature::Formatting => "Prettier",
            ProjectFeature::TailwindCss => "Tailwind CSS",
            ProjectFeature::MaterialUi => "Material-UI",
            ProjectFeature::ReactRouter => "React Router",
            ProjectFeature::StyledComponents => "Styled Components",
            ProjectFeature::Storybook => "Storybook",
            ProjectFeature::Docker => "Docker",
            ProjectFeature::GithubActions => "GitHub Actions",
            ProjectFeature::GitHooks => "Git Hooks",
        }
    }
}

/// Result of project scaffolding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaffoldResult {
    /// Project root directory
    pub project_root: PathBuf,
    /// Files that were created
    pub created_files: Vec<PathBuf>,
    /// Commands that were executed
    pub executed_commands: Vec<String>,
    /// Next steps for the user
    pub next_steps: Vec<String>,
    /// Total execution time
    pub execution_time: f64,
}

/// Project scaffolding system for creating complete project structures
pub struct ProjectScaffolder {
    file_ops: std::sync::Arc<crate::file_operations::FileOperationsManager>,
    shell_manager: std::sync::Arc<crate::shell_execution::ShellExecutionManager>,
}

impl ProjectScaffolder {
    pub fn new() -> Self {
        Self {
            file_ops: get_file_operations_manager(),
            shell_manager: get_shell_execution_manager(),
        }
    }

    /// Create a new project from template
    pub async fn scaffold_project(&self, config: ScaffoldConfig) -> Result<ScaffoldResult> {
        log_info!("scaffold", "🏗️ Scaffolding {} project: {}", 
                 config.template.name(), config.project_name);

        let start_time = std::time::Instant::now();
        let mut result = ScaffoldResult {
            project_root: config.project_root.clone(),
            created_files: Vec::new(),
            executed_commands: Vec::new(),
            next_steps: Vec::new(),
            execution_time: 0.0,
        };

        // Create project directory
        self.create_project_directory(&config, &mut result).await?;

        // Initialize package.json and dependencies
        self.initialize_package_json(&config, &mut result).await?;

        // Install dependencies
        self.install_dependencies(&config, &mut result).await?;

        // Create project structure
        self.create_project_structure(&config, &mut result).await?;

        // Generate configuration files
        self.generate_config_files(&config, &mut result).await?;

        // Add selected features
        self.add_features(&config, &mut result).await?;

        // Initialize git repository
        self.initialize_git(&config, &mut result).await?;

        // Generate initial code
        self.generate_initial_code(&config, &mut result).await?;

        result.execution_time = start_time.elapsed().as_secs_f64();
        log_info!("scaffold", "✅ Project scaffolding completed in {:.2}s", result.execution_time);

        Ok(result)
    }

    /// Create the project directory structure
    async fn create_project_directory(&self, config: &ScaffoldConfig, result: &mut ScaffoldResult) -> Result<()> {
        log_debug!("scaffold", "📁 Creating project directory: {}", config.project_root.display());

        let mkdir_cmd = ShellParams {
            command: format!("mkdir -p \"{}\"", config.project_root.display()),
            description: Some("Create project root directory".to_string()),
            directory: config.project_root.parent().map(|p| p.to_path_buf()),
            timeout_seconds: Some(30),
        };

        let shell_result = self.shell_manager.execute_command(mkdir_cmd).await?;
        if shell_result.success {
            result.executed_commands.push(shell_result.command);
        }

        Ok(())
    }

    /// Initialize package.json with dependencies
    async fn initialize_package_json(&self, config: &ScaffoldConfig, result: &mut ScaffoldResult) -> Result<()> {
        log_debug!("scaffold", "📦 Initializing package.json");

        let package_json = self.generate_package_json(config)?;
        let package_json_path = config.project_root.join("package.json");

        let write_params = WriteFileParams {
            file_path: package_json_path.clone(),
            content: package_json,
        };

        self.file_ops.write_file(write_params).await?;
        result.created_files.push(package_json_path);

        Ok(())
    }

    /// Install project dependencies
    async fn install_dependencies(&self, config: &ScaffoldConfig, result: &mut ScaffoldResult) -> Result<()> {
        log_debug!("scaffold", "📥 Installing dependencies");

        // Approve npm commands for the workflow
        self.shell_manager.approve_pattern("npm *").await;

        let install_cmd = ShellParams {
            command: "npm install".to_string(),
            description: Some("Install project dependencies".to_string()),
            directory: Some(config.project_root.clone()),
            timeout_seconds: Some(300), // 5 minutes for npm install
        };

        let shell_result = self.shell_manager.execute_command(install_cmd).await?;
        if shell_result.success {
            result.executed_commands.push(shell_result.command);
            log_info!("scaffold", "✅ Dependencies installed successfully");
        } else {
            log_warn!("scaffold", "⚠️ Dependency installation had issues: {}", shell_result.stderr);
        }

        Ok(())
    }

    /// Create the basic project structure
    async fn create_project_structure(&self, config: &ScaffoldConfig, result: &mut ScaffoldResult) -> Result<()> {
        log_debug!("scaffold", "🏗️ Creating project structure");

        let directories = self.get_template_directories(&config.template);
        
        for dir in directories {
            let full_path = config.project_root.join(dir);
            let mkdir_cmd = ShellParams {
                command: format!("mkdir -p \"{}\"", full_path.display()),
                description: Some(format!("Create directory: {}", dir)),
                directory: Some(config.project_root.clone()),
                timeout_seconds: Some(30),
            };

            if let Ok(shell_result) = self.shell_manager.execute_command(mkdir_cmd).await {
                if shell_result.success {
                    result.executed_commands.push(shell_result.command);
                }
            }
        }

        Ok(())
    }

    /// Generate configuration files (tsconfig.json, vite.config.ts, etc.)
    async fn generate_config_files(&self, config: &ScaffoldConfig, result: &mut ScaffoldResult) -> Result<()> {
        log_debug!("scaffold", "⚙️ Generating configuration files");

        // TypeScript configuration
        if self.template_uses_typescript(&config.template) {
            let tsconfig_path = config.project_root.join("tsconfig.json");
            let tsconfig_content = self.generate_tsconfig(&config.template)?;
            
            let write_params = WriteFileParams {
                file_path: tsconfig_path.clone(),
                content: tsconfig_content,
            };

            self.file_ops.write_file(write_params).await?;
            result.created_files.push(tsconfig_path);
        }

        // Build tool configuration (Vite, Webpack, etc.)
        match config.template {
            ProjectTemplate::ReactTypeScript | ProjectTemplate::ReactRedux => {
                let vite_config_path = config.project_root.join("vite.config.ts");
                let vite_config = self.generate_vite_config()?;
                
                let write_params = WriteFileParams {
                    file_path: vite_config_path.clone(),
                    content: vite_config,
                };

                self.file_ops.write_file(write_params).await?;
                result.created_files.push(vite_config_path);
            }
            _ => {}
        }

        Ok(())
    }

    /// Add selected features to the project
    async fn add_features(&self, config: &ScaffoldConfig, result: &mut ScaffoldResult) -> Result<()> {
        log_debug!("scaffold", "✨ Adding project features");

        for feature in &config.features {
            match self.add_feature(feature, config, result).await {
                Ok(_) => log_debug!("scaffold", "✅ Added feature: {}", feature.name()),
                Err(e) => log_warn!("scaffold", "⚠️ Failed to add feature {}: {}", feature.name(), e),
            }
        }

        Ok(())
    }

    /// Initialize git repository
    async fn initialize_git(&self, config: &ScaffoldConfig, result: &mut ScaffoldResult) -> Result<()> {
        log_debug!("scaffold", "🔧 Initializing git repository");

        // Approve git commands for the workflow
        self.shell_manager.approve_pattern("git *").await;

        let git_init_cmd = ShellParams {
            command: "git init".to_string(),
            description: Some("Initialize git repository".to_string()),
            directory: Some(config.project_root.clone()),
            timeout_seconds: Some(30),
        };

        if let Ok(shell_result) = self.shell_manager.execute_command(git_init_cmd).await {
            if shell_result.success {
                result.executed_commands.push(shell_result.command);

                // Create .gitignore
                let gitignore_path = config.project_root.join(".gitignore");
                let gitignore_content = self.generate_gitignore(&config.template)?;
                
                let write_params = WriteFileParams {
                    file_path: gitignore_path.clone(),
                    content: gitignore_content,
                };

                self.file_ops.write_file(write_params).await?;
                result.created_files.push(gitignore_path);
            }
        }

        Ok(())
    }

    /// Generate initial application code
    async fn generate_initial_code(&self, config: &ScaffoldConfig, result: &mut ScaffoldResult) -> Result<()> {
        log_debug!("scaffold", "💻 Generating initial code");

        match config.template {
            ProjectTemplate::ReactTypeScript | ProjectTemplate::ReactRedux => {
                self.generate_react_app_code(config, result).await?;
            }
            ProjectTemplate::NodeTypeScript | ProjectTemplate::ExpressApi => {
                self.generate_node_app_code(config, result).await?;
            }
            _ => {
                log_debug!("scaffold", "📝 Basic template, skipping code generation");
            }
        }

        // Generate next steps
        result.next_steps = self.generate_next_steps(&config.template);

        Ok(())
    }

    /// Generate React application code
    async fn generate_react_app_code(&self, config: &ScaffoldConfig, result: &mut ScaffoldResult) -> Result<()> {
        // For now, we'll generate basic React code without using the code generation engine
        // This can be enhanced later to use the full code generation capabilities
        
        let app_tsx_path = config.project_root.join("src").join("App.tsx");
        let app_tsx_content = self.generate_react_app_component(&config.project_name)?;
        
        let write_params = WriteFileParams {
            file_path: app_tsx_path.clone(),
            content: app_tsx_content,
        };

        self.file_ops.write_file(write_params).await?;
        result.created_files.push(app_tsx_path);

        // Generate main entry point
        let main_tsx_path = config.project_root.join("src").join("main.tsx");
        let main_tsx_content = self.generate_main_tsx(&config.project_name)?;
        
        let write_params = WriteFileParams {
            file_path: main_tsx_path.clone(),
            content: main_tsx_content,
        };

        self.file_ops.write_file(write_params).await?;
        result.created_files.push(main_tsx_path);

        // Generate index.html
        let index_html_path = config.project_root.join("index.html");
        let index_html_content = self.generate_index_html(&config.project_name)?;
        
        let write_params = WriteFileParams {
            file_path: index_html_path.clone(),
            content: index_html_content,
        };

        self.file_ops.write_file(write_params).await?;
        result.created_files.push(index_html_path);

        // Generate basic CSS files
        let app_css_path = config.project_root.join("src").join("App.css");
        let app_css_content = self.generate_app_css()?;
        
        let write_params = WriteFileParams {
            file_path: app_css_path.clone(),
            content: app_css_content,
        };

        self.file_ops.write_file(write_params).await?;
        result.created_files.push(app_css_path);

        let index_css_path = config.project_root.join("src").join("index.css");
        let index_css_content = self.generate_index_css()?;
        
        let write_params = WriteFileParams {
            file_path: index_css_path.clone(),
            content: index_css_content,
        };

        self.file_ops.write_file(write_params).await?;
        result.created_files.push(index_css_path);

        Ok(())
    }

    /// Generate Node.js application code
    async fn generate_node_app_code(&self, config: &ScaffoldConfig, result: &mut ScaffoldResult) -> Result<()> {
        let src_index_path = config.project_root.join("src").join("index.ts");
        let index_content = match config.template {
            ProjectTemplate::ExpressApi => self.generate_express_index()?,
            _ => self.generate_node_index()?,
        };
        
        let write_params = WriteFileParams {
            file_path: src_index_path.clone(),
            content: index_content,
        };

        self.file_ops.write_file(write_params).await?;
        result.created_files.push(src_index_path);

        Ok(())
    }

    /// Add a specific feature to the project
    async fn add_feature(&self, feature: &ProjectFeature, config: &ScaffoldConfig, result: &mut ScaffoldResult) -> Result<()> {
        match feature {
            ProjectFeature::Testing => {
                // Will be added via package.json dependencies
                let jest_config_path = config.project_root.join("jest.config.js");
                let jest_config = self.generate_jest_config()?;
                
                let write_params = WriteFileParams {
                    file_path: jest_config_path.clone(),
                    content: jest_config,
                };

                self.file_ops.write_file(write_params).await?;
                result.created_files.push(jest_config_path);
            }
            ProjectFeature::TailwindCss => {
                let tailwind_config_path = config.project_root.join("tailwind.config.js");
                let tailwind_config = self.generate_tailwind_config()?;
                
                let write_params = WriteFileParams {
                    file_path: tailwind_config_path.clone(),
                    content: tailwind_config,
                };

                self.file_ops.write_file(write_params).await?;
                result.created_files.push(tailwind_config_path);
            }
            ProjectFeature::Docker => {
                let dockerfile_path = config.project_root.join("Dockerfile");
                let dockerfile = self.generate_dockerfile(&config.template)?;
                
                let write_params = WriteFileParams {
                    file_path: dockerfile_path.clone(),
                    content: dockerfile,
                };

                self.file_ops.write_file(write_params).await?;
                result.created_files.push(dockerfile_path);
            }
            _ => {
                log_debug!("scaffold", "📝 Feature {} handled via dependencies", feature.name());
            }
        }

        Ok(())
    }

    /// Get directories to create for a template
    fn get_template_directories(&self, template: &ProjectTemplate) -> Vec<&'static str> {
        match template {
            ProjectTemplate::ReactTypeScript | ProjectTemplate::ReactRedux => {
                vec!["src", "src/components", "src/hooks", "src/utils", "src/types", "public"]
            }
            ProjectTemplate::NodeTypeScript | ProjectTemplate::ExpressApi => {
                vec!["src", "src/routes", "src/middleware", "src/utils", "src/types", "tests"]
            }
            ProjectTemplate::NextJs => {
                vec!["src", "src/app", "src/components", "src/lib", "src/types", "public"]
            }
            _ => vec!["src", "src/types", "tests"]
        }
    }

    /// Check if template uses TypeScript
    fn template_uses_typescript(&self, template: &ProjectTemplate) -> bool {
        match template {
            ProjectTemplate::ReactTypeScript 
            | ProjectTemplate::ReactRedux 
            | ProjectTemplate::NodeTypeScript 
            | ProjectTemplate::ExpressApi 
            | ProjectTemplate::NextJs
            | ProjectTemplate::VueTypeScript
            | ProjectTemplate::TypeScriptLibrary => true,
        }
    }

    /// Generate package.json content
    fn generate_package_json(&self, config: &ScaffoldConfig) -> Result<String> {
        let mut dependencies = HashMap::new();
        let mut dev_dependencies = HashMap::new();
        let mut scripts = HashMap::new();

        // Base scripts
        scripts.insert("dev", "vite");
        scripts.insert("build", "vite build");
        scripts.insert("preview", "vite preview");

        match config.template {
            ProjectTemplate::ReactTypeScript | ProjectTemplate::ReactRedux => {
                dependencies.insert("react", "^18.3.1");
                dependencies.insert("react-dom", "^18.3.1");
                
                dev_dependencies.insert("@types/react", "^18.3.14");
                dev_dependencies.insert("@types/react-dom", "^18.3.1");
                dev_dependencies.insert("@vitejs/plugin-react", "^4.3.4");
                dev_dependencies.insert("typescript", "~5.6.2");
                dev_dependencies.insert("vite", "^6.0.3");

                if matches!(config.template, ProjectTemplate::ReactRedux) {
                    dependencies.insert("@reduxjs/toolkit", "^2.5.0");
                    dependencies.insert("react-redux", "^9.1.2");
                    dev_dependencies.insert("@types/react-redux", "^7.1.34");
                }
            }
            ProjectTemplate::NodeTypeScript => {
                dev_dependencies.insert("typescript", "~5.6.2");
                dev_dependencies.insert("@types/node", "^22.10.2");
                dev_dependencies.insert("tsx", "^4.19.2");
                
                scripts.insert("dev", "tsx src/index.ts");
                scripts.insert("build", "tsc");
                scripts.insert("start", "node dist/index.js");
            }
            ProjectTemplate::ExpressApi => {
                dependencies.insert("express", "^4.21.2");
                dependencies.insert("cors", "^2.8.5");
                
                dev_dependencies.insert("typescript", "~5.6.2");
                dev_dependencies.insert("@types/node", "^22.10.2");
                dev_dependencies.insert("@types/express", "^5.0.0");
                dev_dependencies.insert("@types/cors", "^2.8.17");
                dev_dependencies.insert("tsx", "^4.19.2");
                
                scripts.insert("dev", "tsx src/index.ts");
                scripts.insert("build", "tsc");
                scripts.insert("start", "node dist/index.js");
            }
            _ => {}
        }

        // Add feature dependencies
        for feature in &config.features {
            match feature {
                ProjectFeature::Testing => {
                    dev_dependencies.insert("@jest/globals", "^29.7.0");
                    dev_dependencies.insert("jest", "^29.7.0");
                    dev_dependencies.insert("ts-jest", "^29.2.5");
                    scripts.insert("test", "jest");
                }
                ProjectFeature::Linting => {
                    dev_dependencies.insert("eslint", "^9.17.0");
                    dev_dependencies.insert("@typescript-eslint/eslint-plugin", "^8.18.2");
                    dev_dependencies.insert("@typescript-eslint/parser", "^8.18.2");
                    scripts.insert("lint", "eslint src/**/*.{ts,tsx}");
                }
                ProjectFeature::Formatting => {
                    dev_dependencies.insert("prettier", "^3.4.2");
                    scripts.insert("format", "prettier --write src/**/*.{ts,tsx}");
                }
                ProjectFeature::TailwindCss => {
                    dev_dependencies.insert("tailwindcss", "^3.4.17");
                    dev_dependencies.insert("autoprefixer", "^10.4.20");
                    dev_dependencies.insert("postcss", "^8.5.11");
                }
                ProjectFeature::MaterialUi => {
                    dependencies.insert("@mui/material", "^6.3.0");
                    dependencies.insert("@emotion/react", "^11.14.0");
                    dependencies.insert("@emotion/styled", "^11.14.0");
                }
                ProjectFeature::ReactRouter => {
                    dependencies.insert("react-router-dom", "^7.1.3");
                    dev_dependencies.insert("@types/react-router-dom", "^5.3.3");
                }
                _ => {}
            }
        }

        let deps_json = dependencies.iter()
            .map(|(k, v)| format!("    \"{}\": \"{}\"", k, v))
            .collect::<Vec<_>>()
            .join(",\n");

        let dev_deps_json = dev_dependencies.iter()
            .map(|(k, v)| format!("    \"{}\": \"{}\"", k, v))
            .collect::<Vec<_>>()
            .join(",\n");

        let scripts_json = scripts.iter()
            .map(|(k, v)| format!("    \"{}\": \"{}\"", k, v))
            .collect::<Vec<_>>()
            .join(",\n");

        let author_field = config.author.as_ref()
            .map(|a| format!(",\n  \"author\": \"{}\"", a))
            .unwrap_or_default();

        let description_field = config.description.as_ref()
            .map(|d| format!(",\n  \"description\": \"{}\"", d))
            .unwrap_or_default();

        let license_field = config.license.as_ref()
            .map(|l| format!(",\n  \"license\": \"{}\"", l))
            .unwrap_or_else(|| ",\n  \"license\": \"MIT\"".to_string());

        Ok(format!(
            r#"{{
  "name": "{}",
  "version": "1.0.0",
  "type": "module"{}{}{},"
  "scripts": {{
{}
  }},
  "dependencies": {{
{}
  }},
  "devDependencies": {{
{}
  }}
}}"#,
            config.project_name, description_field, author_field, license_field,
            scripts_json, deps_json, dev_deps_json
        ))
    }

    /// Generate TypeScript configuration
    fn generate_tsconfig(&self, template: &ProjectTemplate) -> Result<String> {
        match template {
            ProjectTemplate::ReactTypeScript | ProjectTemplate::ReactRedux => {
                Ok(r#"{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "noUncheckedSideEffectImports": true
  },
  "include": ["src"]
}"#.to_string())
            }
            _ => {
                Ok(r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}"#.to_string())
            }
        }
    }

    /// Generate Vite configuration
    fn generate_vite_config(&self) -> Result<String> {
        Ok(r#"import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
    open: true
  }
})
"#.to_string())
    }

    /// Generate React App component
    fn generate_react_app_component(&self, project_name: &str) -> Result<String> {
        Ok(format!(r#"import {{ useState }} from 'react'
import './App.css'

function App() {{
  const [count, setCount] = useState(0)

  return (
    <div className="App">
      <header className="App-header">
        <h1>{}</h1>
        <div className="card">
          <button onClick={{() => setCount((count) => count + 1)}}>
            count is {{count}}
          </button>
          <p>
            Edit <code>src/App.tsx</code> and save to test HMR
          </p>
        </div>
        <p className="read-the-docs">
          Click on the Vite and React logos to learn more
        </p>
      </header>
    </div>
  )
}}

export default App
"#, project_name))
    }

    /// Generate main.tsx for React apps
    fn generate_main_tsx(&self, _project_name: &str) -> Result<String> {
        Ok(format!(r#"import {{ StrictMode }} from 'react'
import {{ createRoot }} from 'react-dom/client'
import App from './App'
import './index.css'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>
)
"#))
    }

    /// Generate index.html for React apps
    fn generate_index_html(&self, project_name: &str) -> Result<String> {
        Ok(format!(r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{}</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
"#, project_name))
    }

    /// Generate Express.js index
    fn generate_express_index(&self) -> Result<String> {
        Ok(r#"import express from 'express';
import cors from 'cors';

const app = express();
const PORT = process.env.PORT || 3000;

// Middleware
app.use(cors());
app.use(express.json());

// Routes
app.get('/', (req, res) => {
  res.json({ message: 'Hello World!' });
});

app.get('/health', (req, res) => {
  res.json({ status: 'OK', timestamp: new Date().toISOString() });
});

// Start server
app.listen(PORT, () => {
  console.log(`Server running on port ${PORT}`);
});
"#.to_string())
    }

    /// Generate Node.js index
    fn generate_node_index(&self) -> Result<String> {
        Ok(r#"console.log('Hello, TypeScript Node.js!');

// Main application logic here
async function main() {
  console.log('Application started');
  
  // Your code here
}

main().catch(console.error);
"#.to_string())
    }

    /// Generate .gitignore
    fn generate_gitignore(&self, template: &ProjectTemplate) -> Result<String> {
        Ok(r#"# Dependencies
node_modules/
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# Build outputs
dist/
build/
*.tsbuildinfo

# Environment variables
.env
.env.local
.env.development.local
.env.test.local
.env.production.local

# IDE
.vscode/
.idea/
*.swp
*.swo

# OS generated files
.DS_Store
.DS_Store?
._*
.Spotlight-V100
.Trashes
ehthumbs.db
Thumbs.db

# Logs
logs
*.log

# Coverage directory used by tools like istanbul
coverage/
*.lcov

# Dependency directories
jspm_packages/

# Optional npm cache directory
.npm

# Optional eslint cache
.eslintcache

# Output of 'npm pack'
*.tgz

# Yarn Integrity file
.yarn-integrity
"#.to_string())
    }

    /// Generate Jest configuration
    fn generate_jest_config(&self) -> Result<String> {
        Ok(r#"module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  roots: ['<rootDir>/src'],
  testMatch: ['**/__tests__/**/*.ts', '**/?(*.)+(spec|test).ts'],
  transform: {
    '^.+\\.ts$': 'ts-jest',
  },
  collectCoverageFrom: [
    'src/**/*.ts',
    '!src/**/*.d.ts',
  ],
};
"#.to_string())
    }

    /// Generate Tailwind CSS configuration
    fn generate_tailwind_config(&self) -> Result<String> {
        Ok(r#"/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}
"#.to_string())
    }

    /// Generate Dockerfile
    fn generate_dockerfile(&self, template: &ProjectTemplate) -> Result<String> {
        match template {
            ProjectTemplate::ReactTypeScript | ProjectTemplate::ReactRedux => {
                Ok(r#"FROM node:18-alpine

WORKDIR /app

COPY package*.json ./
RUN npm ci --only=production

COPY . .
RUN npm run build

EXPOSE 3000

CMD ["npm", "run", "preview"]
"#.to_string())
            }
            _ => {
                Ok(r#"FROM node:18-alpine

WORKDIR /app

COPY package*.json ./
RUN npm ci --only=production

COPY . .
RUN npm run build

EXPOSE 3000

CMD ["npm", "start"]
"#.to_string())
            }
        }
    }

    /// Generate next steps for the user
    fn generate_next_steps(&self, template: &ProjectTemplate) -> Vec<String> {
        let mut steps = vec![
            "Navigate to the project directory".to_string(),
            "Run 'npm run dev' to start the development server".to_string(),
        ];

        match template {
            ProjectTemplate::ReactTypeScript | ProjectTemplate::ReactRedux => {
                steps.push("Open http://localhost:3000 in your browser".to_string());
                steps.push("Start building your React components in src/components/".to_string());
            }
            ProjectTemplate::ExpressApi => {
                steps.push("API will be available at http://localhost:3000".to_string());
                steps.push("Add your routes in src/routes/".to_string());
            }
            _ => {
                steps.push("Start developing your application".to_string());
            }
        }

        steps.push("Run 'npm test' to execute tests".to_string());
        steps.push("Run 'npm run build' to create a production build".to_string());

        steps
    }

    /// Generate App.css
    fn generate_app_css(&self) -> Result<String> {
        Ok(r#".App {
  max-width: 1280px;
  margin: 0 auto;
  padding: 2rem;
  text-align: center;
}

.App-header {
  background-color: #282c34;
  padding: 20px;
  color: white;
  border-radius: 8px;
}

.card {
  padding: 2em;
}

.read-the-docs {
  color: #888;
}

button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  background-color: #1a1a1a;
  color: white;
  cursor: pointer;
  transition: border-color 0.25s;
}

button:hover {
  border-color: #646cff;
}

button:focus,
button:focus-visible {
  outline: 4px auto -webkit-focus-ring-color;
}
"#.to_string())
    }

    /// Generate index.css
    fn generate_index_css(&self) -> Result<String> {
        Ok(r#":root {
  font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
  line-height: 1.5;
  font-weight: 400;

  color-scheme: light dark;
  color: rgba(255, 255, 255, 0.87);
  background-color: #242424;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}
a:hover {
  color: #535bf2;
}

body {
  margin: 0;
  display: flex;
  place-items: center;
  min-width: 320px;
  min-height: 100vh;
}

h1 {
  font-size: 3.2em;
  line-height: 1.1;
}

#root {
  max-width: 1280px;
  margin: 0 auto;
  padding: 2rem;
  text-align: center;
}

@media (prefers-color-scheme: light) {
  :root {
    color: #213547;
    background-color: #ffffff;
  }
  a:hover {
    color: #747bff;
  }
  button {
    background-color: #f9f9f9;
  }
}
"#.to_string())
    }
}

/// Global singleton instance
static PROJECT_SCAFFOLDER: once_cell::sync::Lazy<std::sync::Arc<ProjectScaffolder>> = 
    once_cell::sync::Lazy::new(|| {
        std::sync::Arc::new(ProjectScaffolder::new())
    });

/// Get global project scaffolder instance
pub fn get_project_scaffolder() -> std::sync::Arc<ProjectScaffolder> {
    PROJECT_SCAFFOLDER.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_project_template_names() {
        assert_eq!(ProjectTemplate::ReactTypeScript.name(), "React TypeScript");
        assert_eq!(ProjectTemplate::ExpressApi.name(), "Express.js API");
    }

    #[test]
    fn test_package_json_generation() {
        let temp_dir = TempDir::new().unwrap();
        let scaffolder = ProjectScaffolder::new();
        
        let config = ScaffoldConfig {
            project_name: "test-project".to_string(),
            project_root: temp_dir.path().to_path_buf(),
            template: ProjectTemplate::ReactTypeScript,
            features: vec![ProjectFeature::Testing],
            author: Some("Test Author".to_string()),
            description: Some("Test project".to_string()),
            license: Some("MIT".to_string()),
        };

        let package_json = scaffolder.generate_package_json(&config).unwrap();
        assert!(package_json.contains("test-project"));
        assert!(package_json.contains("Test Author"));
        assert!(package_json.contains("react"));
        assert!(package_json.contains("jest"));
    }
}