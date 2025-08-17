use anyhow::Result;
use std::path::PathBuf;

use crate::project_scaffolding::{get_project_scaffolder, ScaffoldConfig, ProjectTemplate, ProjectFeature};
use crate::dependency_manager::{get_dependency_manager, DependencyType};
use crate::code_generation::get_code_generation_engine;
use crate::file_operations::get_file_operations_manager;
use crate::shell_execution::get_shell_execution_manager;
use crate::logger::{log_info, log_debug, log_warn, log_error};

/// Comprehensive integration test for multi-stage development workflow
pub struct MultiStageWorkflowTest {
    project_root: PathBuf,
}

impl MultiStageWorkflowTest {
    pub fn new() -> Result<Self> {
        use std::env;
        let temp_dir = env::temp_dir();
        let project_root = temp_dir.join("cai-integration-test");
        
        // Ensure the directory exists
        std::fs::create_dir_all(&project_root)?;
        
        Ok(Self {
            project_root,
        })
    }

    /// Execute the complete 10-stage development workflow
    pub async fn run_complete_workflow(&self) -> Result<WorkflowTestResult> {
        log_info!("integration_test", "🚀 Starting 10-stage development workflow test");
        let start_time = std::time::Instant::now();
        
        let mut result = WorkflowTestResult::new();

        // Stage 1: Project Scaffolding
        log_info!("integration_test", "📋 Stage 1: Project Scaffolding");
        self.stage_1_project_scaffolding(&mut result).await?;

        // Stage 2: Dependency Management
        log_info!("integration_test", "📦 Stage 2: Dependency Management");
        self.stage_2_dependency_management(&mut result).await?;

        // Stage 3: Core Component Generation
        log_info!("integration_test", "⚛️ Stage 3: Core Component Generation");
        self.stage_3_core_component_generation(&mut result).await?;

        // Stage 4: State Management Setup
        log_info!("integration_test", "🏪 Stage 4: State Management Setup");
        self.stage_4_state_management(&mut result).await?;

        // Stage 5: API Integration
        log_info!("integration_test", "🌐 Stage 5: API Integration");
        self.stage_5_api_integration(&mut result).await?;

        // Stage 6: Styling and Theme
        log_info!("integration_test", "🎨 Stage 6: Styling and Theme");
        self.stage_6_styling_theme(&mut result).await?;

        // Stage 7: Testing Infrastructure
        log_info!("integration_test", "🧪 Stage 7: Testing Infrastructure");
        self.stage_7_testing_infrastructure(&mut result).await?;

        // Stage 8: Build Configuration
        log_info!("integration_test", "🔧 Stage 8: Build Configuration");
        self.stage_8_build_configuration(&mut result).await?;

        // Stage 9: Documentation Generation
        log_info!("integration_test", "📚 Stage 9: Documentation Generation");
        self.stage_9_documentation(&mut result).await?;

        // Stage 10: Final Validation and Build
        log_info!("integration_test", "✅ Stage 10: Final Validation and Build");
        self.stage_10_final_validation(&mut result).await?;

        result.total_execution_time = start_time.elapsed().as_secs_f64();
        log_info!("integration_test", "🎉 10-stage workflow completed in {:.2}s", result.total_execution_time);

        Ok(result)
    }

    /// Stage 1: Create project structure and basic scaffolding
    async fn stage_1_project_scaffolding(&self, result: &mut WorkflowTestResult) -> Result<()> {
        let scaffolder = get_project_scaffolder();
        
        let config = ScaffoldConfig {
            project_name: "test-webapp".to_string(),
            project_root: self.project_root.clone(),
            template: ProjectTemplate::ReactTypeScript,
            features: vec![
                ProjectFeature::Testing,
                ProjectFeature::Linting,
                ProjectFeature::Formatting,
                ProjectFeature::TailwindCss,
            ],
            author: Some("CAI Test Suite".to_string()),
            description: Some("Multi-stage development test application".to_string()),
            license: Some("MIT".to_string()),
        };

        let scaffold_result = scaffolder.scaffold_project(config).await?;
        let files_created = scaffold_result.created_files.len();
        result.add_stage_result(1, "Project Scaffolding", files_created, true);
        result.created_files.extend(scaffold_result.created_files);
        
        log_debug!("integration_test", "✅ Stage 1 completed: {} files created", files_created);
        Ok(())
    }

    /// Stage 2: Add additional dependencies and modify package.json
    async fn stage_2_dependency_management(&self, result: &mut WorkflowTestResult) -> Result<()> {
        let dep_manager = get_dependency_manager();
        
        // Add additional production dependencies
        let prod_deps = vec![
            ("axios", "^1.6.0", DependencyType::Production),
            ("react-router-dom", "^6.20.0", DependencyType::Production),
            ("zustand", "^4.4.0", DependencyType::Production),
        ];

        let dep_result = dep_manager.add_dependencies_from_specs(&self.project_root, &prod_deps).await?;
        
        // Add development dependencies
        dep_manager.add_dependency(&self.project_root, "@types/node", "^20.0.0", DependencyType::Development).await?;
        dep_manager.add_dependency(&self.project_root, "vite-plugin-eslint", "^1.8.0", DependencyType::Development).await?;

        // Add custom scripts
        dep_manager.add_script(&self.project_root, "test:watch", "jest --watch").await?;
        dep_manager.add_script(&self.project_root, "build:analyze", "vite build --mode analyze").await?;

        result.add_stage_result(2, "Dependency Management", dep_result.added_dependencies.len(), true);
        
        log_debug!("integration_test", "✅ Stage 2 completed: {} dependencies added", dep_result.added_dependencies.len());
        Ok(())
    }

    /// Stage 3: Generate core React components
    async fn stage_3_core_component_generation(&self, result: &mut WorkflowTestResult) -> Result<()> {
        let code_gen = get_code_generation_engine().await?;
        
        // Generate Header component
        let header_result = code_gen.generate_react_component(
            "Header",
            "main navigation header with logo and menu items",
            &[("title", "string"), ("menuItems", "MenuItem[]")],
            self.project_root.clone()
        ).await?;

        // Generate Footer component
        let footer_result = code_gen.generate_react_component(
            "Footer",
            "application footer with links and copyright",
            &[("links", "FooterLink[]"), ("copyright", "string")],
            self.project_root.clone()
        ).await?;

        // Generate UserCard component
        let user_card_result = code_gen.generate_react_component(
            "UserCard",
            "displays user information in a card format",
            &[("user", "User"), ("onEdit", "() => void")],
            self.project_root.clone()
        ).await?;

        if let Some(file) = &header_result.target_file {
            result.created_files.push(file.clone());
        }
        if let Some(file) = &footer_result.target_file {
            result.created_files.push(file.clone());
        }
        if let Some(file) = &user_card_result.target_file {
            result.created_files.push(file.clone());
        }

        result.add_stage_result(3, "Core Component Generation", 3, true);
        
        log_debug!("integration_test", "✅ Stage 3 completed: 3 components generated");
        Ok(())
    }

    /// Stage 4: Setup state management with Zustand
    async fn stage_4_state_management(&self, result: &mut WorkflowTestResult) -> Result<()> {
        let file_ops = get_file_operations_manager();
        
        // Create store directory
        let store_dir = self.project_root.join("src").join("store");
        std::fs::create_dir_all(&store_dir)?;

        // Generate user store
        let user_store_content = r#"import { create } from 'zustand';

export interface User {
  id: string;
  name: string;
  email: string;
  avatar?: string;
}

interface UserState {
  users: User[];
  currentUser: User | null;
  loading: boolean;
  addUser: (user: Omit<User, 'id'>) => void;
  updateUser: (id: string, updates: Partial<User>) => void;
  deleteUser: (id: string) => void;
  setCurrentUser: (user: User | null) => void;
  setLoading: (loading: boolean) => void;
}

export const useUserStore = create<UserState>((set) => ({
  users: [],
  currentUser: null,
  loading: false,
  addUser: (user) => set((state) => ({
    users: [...state.users, { ...user, id: crypto.randomUUID() }]
  })),
  updateUser: (id, updates) => set((state) => ({
    users: state.users.map(user => user.id === id ? { ...user, ...updates } : user)
  })),
  deleteUser: (id) => set((state) => ({
    users: state.users.filter(user => user.id !== id)
  })),
  setCurrentUser: (currentUser) => set({ currentUser }),
  setLoading: (loading) => set({ loading }),
}));
"#;

        let user_store_path = store_dir.join("userStore.ts");
        file_ops.write_file(crate::file_operations::WriteFileParams {
            file_path: user_store_path.clone(),
            content: user_store_content.to_string(),
        }).await?;

        result.created_files.push(user_store_path);
        result.add_stage_result(4, "State Management Setup", 1, true);
        
        log_debug!("integration_test", "✅ Stage 4 completed: State management configured");
        Ok(())
    }

    /// Stage 5: API integration setup
    async fn stage_5_api_integration(&self, result: &mut WorkflowTestResult) -> Result<()> {
        let file_ops = get_file_operations_manager();
        
        // Create API directory
        let api_dir = self.project_root.join("src").join("api");
        std::fs::create_dir_all(&api_dir)?;

        // Generate API client
        let api_client_content = r#"import axios from 'axios';

const API_BASE_URL = process.env.VITE_API_URL || 'http://localhost:3001/api';

export const apiClient = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

// Request interceptor for auth
apiClient.interceptors.request.use((config) => {
  const token = localStorage.getItem('auth_token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

// Response interceptor for error handling
apiClient.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      localStorage.removeItem('auth_token');
      window.location.href = '/login';
    }
    return Promise.reject(error);
  }
);

export interface ApiResponse<T> {
  data: T;
  message: string;
  success: boolean;
}

export const userApi = {
  getUsers: () => apiClient.get<ApiResponse<User[]>>('/users'),
  getUser: (id: string) => apiClient.get<ApiResponse<User>>(`/users/${id}`),
  createUser: (user: Omit<User, 'id'>) => apiClient.post<ApiResponse<User>>('/users', user),
  updateUser: (id: string, user: Partial<User>) => apiClient.put<ApiResponse<User>>(`/users/${id}`, user),
  deleteUser: (id: string) => apiClient.delete<ApiResponse<void>>(`/users/${id}`),
};
"#;

        let api_client_path = api_dir.join("client.ts");
        file_ops.write_file(crate::file_operations::WriteFileParams {
            file_path: api_client_path.clone(),
            content: api_client_content.to_string(),
        }).await?;

        result.created_files.push(api_client_path);
        result.add_stage_result(5, "API Integration", 1, true);
        
        log_debug!("integration_test", "✅ Stage 5 completed: API integration setup");
        Ok(())
    }

    /// Stage 6: Styling and theme configuration
    async fn stage_6_styling_theme(&self, result: &mut WorkflowTestResult) -> Result<()> {
        let file_ops = get_file_operations_manager();
        
        // Update tailwind config for custom theme
        let tailwind_config_content = r#"/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        primary: {
          50: '#eff6ff',
          500: '#3b82f6',
          600: '#2563eb',
          700: '#1d4ed8',
        },
        secondary: {
          50: '#f0f9ff',
          500: '#06b6d4',
          600: '#0891b2',
        }
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
    },
  },
  plugins: [],
}
"#;

        let tailwind_config_path = self.project_root.join("tailwind.config.js");
        file_ops.write_file(crate::file_operations::WriteFileParams {
            file_path: tailwind_config_path,
            content: tailwind_config_content.to_string(),
        }).await?;

        // Create theme configuration
        let theme_dir = self.project_root.join("src").join("styles");
        std::fs::create_dir_all(&theme_dir)?;

        let theme_content = r#"export const theme = {
  colors: {
    primary: '#3b82f6',
    secondary: '#06b6d4',
    success: '#10b981',
    warning: '#f59e0b',
    error: '#ef4444',
  },
  spacing: {
    xs: '0.5rem',
    sm: '1rem',
    md: '1.5rem',
    lg: '2rem',
    xl: '3rem',
  },
  borderRadius: {
    sm: '0.25rem',
    md: '0.5rem',
    lg: '0.75rem',
    full: '9999px',
  },
};

export type Theme = typeof theme;
"#;

        let theme_path = theme_dir.join("theme.ts");
        file_ops.write_file(crate::file_operations::WriteFileParams {
            file_path: theme_path.clone(),
            content: theme_content.to_string(),
        }).await?;

        result.created_files.push(theme_path);
        result.add_stage_result(6, "Styling and Theme", 2, true);
        
        log_debug!("integration_test", "✅ Stage 6 completed: Styling and theme configured");
        Ok(())
    }

    /// Stage 7: Testing infrastructure
    async fn stage_7_testing_infrastructure(&self, result: &mut WorkflowTestResult) -> Result<()> {
        let code_gen = get_code_generation_engine().await?;
        
        // Generate test for UserCard component
        let user_card_test_path = self.project_root.join("src").join("components").join("UserCard.test.tsx");
        let test_result = code_gen.generate_test_file(&user_card_test_path, "unit", self.project_root.clone()).await?;

        if let Some(file) = &test_result.target_file {
            result.created_files.push(file.clone());
        }

        result.add_stage_result(7, "Testing Infrastructure", 1, true);
        
        log_debug!("integration_test", "✅ Stage 7 completed: Testing infrastructure setup");
        Ok(())
    }

    /// Stage 8: Build configuration optimization
    async fn stage_8_build_configuration(&self, result: &mut WorkflowTestResult) -> Result<()> {
        let file_ops = get_file_operations_manager();
        
        // Update vite config for production optimization
        let vite_config_content = r#"import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ['react', 'react-dom'],
          router: ['react-router-dom'],
          state: ['zustand'],
          api: ['axios'],
        },
      },
    },
    chunkSizeWarningLimit: 1000,
  },
  server: {
    port: 3000,
    open: true,
  },
  preview: {
    port: 4173,
  },
})
"#;

        let vite_config_path = self.project_root.join("vite.config.ts");
        file_ops.write_file(crate::file_operations::WriteFileParams {
            file_path: vite_config_path,
            content: vite_config_content.to_string(),
        }).await?;

        result.add_stage_result(8, "Build Configuration", 1, true);
        
        log_debug!("integration_test", "✅ Stage 8 completed: Build configuration optimized");
        Ok(())
    }

    /// Stage 9: Documentation generation
    async fn stage_9_documentation(&self, result: &mut WorkflowTestResult) -> Result<()> {
        let file_ops = get_file_operations_manager();
        
        // Generate README.md
        let readme_content = r#"# Test WebApp

A modern React TypeScript application built with CAI's multi-stage development workflow.

## Features

- ⚛️ React 18 with TypeScript
- 🎨 Tailwind CSS for styling
- 🏪 Zustand for state management
- 🌐 Axios for API integration
- 🧪 Jest for testing
- 📦 Vite for fast development and building

## Getting Started

### Prerequisites

- Node.js 18 or higher
- npm or yarn

### Installation

```bash
npm install
```

### Development

```bash
npm run dev
```

### Testing

```bash
npm test
npm run test:watch
```

### Building

```bash
npm run build
npm run build:analyze
```

## Project Structure

```
src/
├── components/     # React components
├── store/         # Zustand stores
├── api/           # API integration
├── styles/        # Theme and styling
├── types/         # TypeScript types
└── utils/         # Utility functions
```

## Generated by CAI

This project was generated using CAI's multi-stage development workflow, demonstrating:

1. Project scaffolding
2. Dependency management
3. Component generation
4. State management setup
5. API integration
6. Styling and theming
7. Testing infrastructure
8. Build configuration
9. Documentation generation
10. Final validation

## License

MIT
"#;

        let readme_path = self.project_root.join("README.md");
        file_ops.write_file(crate::file_operations::WriteFileParams {
            file_path: readme_path.clone(),
            content: readme_content.to_string(),
        }).await?;

        result.created_files.push(readme_path);
        result.add_stage_result(9, "Documentation", 1, true);
        
        log_debug!("integration_test", "✅ Stage 9 completed: Documentation generated");
        Ok(())
    }

    /// Stage 10: Final validation and build test
    async fn stage_10_final_validation(&self, result: &mut WorkflowTestResult) -> Result<()> {
        let shell_manager = get_shell_execution_manager();
        
        // Validate package.json structure
        let dep_manager = get_dependency_manager();
        let package_json = dep_manager.read_package_json(&self.project_root).await?;
        
        let has_required_deps = package_json.dependencies.as_ref()
            .map(|deps| deps.contains_key("react") && deps.contains_key("react-dom"))
            .unwrap_or(false);

        if !has_required_deps {
            return Err(anyhow::anyhow!("Missing required dependencies in package.json"));
        }

        // Attempt to install dependencies (dry run)
        shell_manager.approve_pattern("npm *").await;
        let install_result = shell_manager.execute_command(crate::shell_execution::ShellParams {
            command: "npm install --dry-run".to_string(),
            description: Some("Validate dependency installation".to_string()),
            directory: Some(self.project_root.clone()),
            timeout_seconds: Some(60),
        }).await;

        let install_success = install_result.map(|r| r.success).unwrap_or(false);

        // Validate TypeScript configuration
        let tsconfig_exists = self.project_root.join("tsconfig.json").exists();
        let vite_config_exists = self.project_root.join("vite.config.ts").exists();

        let validation_score = [has_required_deps, install_success, tsconfig_exists, vite_config_exists]
            .iter()
            .map(|&b| if b { 1 } else { 0 })
            .sum::<i32>();

        result.add_stage_result(10, "Final Validation", validation_score as usize, validation_score >= 3);
        result.validation_passed = validation_score >= 3;
        
        log_debug!("integration_test", "✅ Stage 10 completed: Validation score {}/4", validation_score);
        Ok(())
    }

    pub fn get_project_path(&self) -> &PathBuf {
        &self.project_root
    }
}

/// Result structure for workflow testing
#[derive(Debug)]
pub struct WorkflowTestResult {
    pub stages: Vec<StageResult>,
    pub created_files: Vec<PathBuf>,
    pub total_execution_time: f64,
    pub validation_passed: bool,
}

impl WorkflowTestResult {
    fn new() -> Self {
        Self {
            stages: Vec::new(),
            created_files: Vec::new(),
            total_execution_time: 0.0,
            validation_passed: false,
        }
    }

    fn add_stage_result(&mut self, stage_number: usize, name: &str, artifacts_created: usize, success: bool) {
        self.stages.push(StageResult {
            stage_number,
            name: name.to_string(),
            artifacts_created,
            success,
        });
    }

    pub fn print_summary(&self) {
        println!("\n🎯 Multi-Stage Workflow Test Results");
        println!("{}", "=".repeat(50));
        println!("⏱️  Total execution time: {:.2}s", self.total_execution_time);
        println!("📁 Total files created: {}", self.created_files.len());
        println!("✅ Validation passed: {}", self.validation_passed);
        println!("\n📋 Stage Results:");
        
        for stage in &self.stages {
            let status = if stage.success { "✅" } else { "❌" };
            println!("   {} Stage {}: {} ({} artifacts)", 
                    status, stage.stage_number, stage.name, stage.artifacts_created);
        }
        
        let successful_stages = self.stages.iter().filter(|s| s.success).count();
        println!("\n🏁 Overall: {}/{} stages completed successfully", 
                successful_stages, self.stages.len());
    }
}

#[derive(Debug)]
pub struct StageResult {
    pub stage_number: usize,
    pub name: String,
    pub artifacts_created: usize,
    pub success: bool,
}

/// Run the complete integration test
pub async fn run_integration_test() -> Result<WorkflowTestResult> {
    let test = MultiStageWorkflowTest::new()?;
    let result = test.run_complete_workflow().await?;
    
    log_info!("integration_test", "🎯 Integration test completed");
    result.print_summary();
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_stage_workflow() {
        let result = run_integration_test().await;
        
        match result {
            Ok(test_result) => {
                assert!(test_result.stages.len() == 10);
                assert!(test_result.created_files.len() > 0);
                
                // At least 8 out of 10 stages should succeed
                let successful_stages = test_result.stages.iter().filter(|s| s.success).count();
                assert!(successful_stages >= 8, "Only {}/10 stages succeeded", successful_stages);
            }
            Err(e) => {
                panic!("Integration test failed: {}", e);
            }
        }
    }
}