//! Project templates for enhanced project generation
//! 
//! This module contains pre-built templates for different languages and frameworks,
//! following industry best practices and providing complete project scaffolding.

use super::*;
use anyhow::Result;

impl ProjectGenerator {
    /// Create Python basic template
    pub(crate) fn create_python_template(&self) -> ProjectTemplate {
        ProjectTemplate {
            name: "python_basic".to_string(),
            language: "python".to_string(),
            framework: None,
            file_templates: HashMap::new(),
            directory_structure: vec![
                "src".to_string(),
                "tests".to_string(),
                "docs".to_string(),
                ".github/workflows".to_string(),
            ],
            dependencies: vec![
                "pytest".to_string(),
                "black".to_string(),
                "pylint".to_string(),
            ],
            quality_requirements: QualityRequirements {
                min_quality_score: 75.0,
                required_files: vec!["README.md".to_string(), "requirements.txt".to_string()],
                required_tests: true,
                required_documentation: true,
                security_checks: vec!["bandit".to_string()],
            },
        }
    }

    /// Create Python Flask API template
    pub(crate) fn create_python_flask_template(&self) -> ProjectTemplate {
        ProjectTemplate {
            name: "python_flask".to_string(),
            language: "python".to_string(),
            framework: Some("flask".to_string()),
            file_templates: HashMap::new(),
            directory_structure: vec![
                "app".to_string(),
                "app/api".to_string(),
                "app/models".to_string(),
                "tests".to_string(),
                "docs".to_string(),
                "migrations".to_string(),
            ],
            dependencies: vec![
                "flask".to_string(),
                "flask-sqlalchemy".to_string(),
                "flask-migrate".to_string(),
                "pytest".to_string(),
                "pytest-flask".to_string(),
            ],
            quality_requirements: QualityRequirements {
                min_quality_score: 80.0,
                required_files: vec!["README.md".to_string(), "requirements.txt".to_string(), "config.py".to_string()],
                required_tests: true,
                required_documentation: true,
                security_checks: vec!["bandit".to_string(), "safety".to_string()],
            },
        }
    }

    /// Create Python FastAPI template
    pub(crate) fn create_python_fastapi_template(&self) -> ProjectTemplate {
        ProjectTemplate {
            name: "python_fastapi".to_string(),
            language: "python".to_string(),
            framework: Some("fastapi".to_string()),
            file_templates: HashMap::new(),
            directory_structure: vec![
                "app".to_string(),
                "app/api".to_string(),
                "app/core".to_string(),
                "app/models".to_string(),
                "app/schemas".to_string(),
                "tests".to_string(),
                "docs".to_string(),
            ],
            dependencies: vec![
                "fastapi".to_string(),
                "uvicorn".to_string(),
                "sqlalchemy".to_string(),
                "pydantic".to_string(),
                "pytest".to_string(),
                "httpx".to_string(),
            ],
            quality_requirements: QualityRequirements {
                min_quality_score: 85.0,
                required_files: vec!["README.md".to_string(), "requirements.txt".to_string(), "pyproject.toml".to_string()],
                required_tests: true,
                required_documentation: true,
                security_checks: vec!["bandit".to_string(), "safety".to_string()],
            },
        }
    }

    /// Create Node.js basic template
    pub(crate) fn create_node_template(&self) -> ProjectTemplate {
        ProjectTemplate {
            name: "node_basic".to_string(),
            language: "javascript".to_string(),
            framework: None,
            file_templates: HashMap::new(),
            directory_structure: vec![
                "src".to_string(),
                "tests".to_string(),
                "docs".to_string(),
                ".github/workflows".to_string(),
            ],
            dependencies: vec![
                "jest".to_string(),
                "eslint".to_string(),
                "prettier".to_string(),
            ],
            quality_requirements: QualityRequirements {
                min_quality_score: 75.0,
                required_files: vec!["README.md".to_string(), "package.json".to_string()],
                required_tests: true,
                required_documentation: true,
                security_checks: vec!["npm audit".to_string()],
            },
        }
    }

    /// Create Node.js Express template
    pub(crate) fn create_node_express_template(&self) -> ProjectTemplate {
        ProjectTemplate {
            name: "node_express".to_string(),
            language: "javascript".to_string(),
            framework: Some("express".to_string()),
            file_templates: HashMap::new(),
            directory_structure: vec![
                "src".to_string(),
                "src/routes".to_string(),
                "src/middleware".to_string(),
                "src/models".to_string(),
                "tests".to_string(),
                "docs".to_string(),
                "public".to_string(),
            ],
            dependencies: vec![
                "express".to_string(),
                "cors".to_string(),
                "helmet".to_string(),
                "jest".to_string(),
                "supertest".to_string(),
            ],
            quality_requirements: QualityRequirements {
                min_quality_score: 80.0,
                required_files: vec!["README.md".to_string(), "package.json".to_string()],
                required_tests: true,
                required_documentation: true,
                security_checks: vec!["npm audit".to_string(), "snyk".to_string()],
            },
        }
    }

    /// Create TypeScript basic template
    pub(crate) fn create_typescript_template(&self) -> ProjectTemplate {
        ProjectTemplate {
            name: "typescript_basic".to_string(),
            language: "typescript".to_string(),
            framework: None,
            file_templates: HashMap::new(),
            directory_structure: vec![
                "src".to_string(),
                "tests".to_string(),
                "docs".to_string(),
                "dist".to_string(),
                ".github/workflows".to_string(),
            ],
            dependencies: vec![
                "typescript".to_string(),
                "ts-node".to_string(),
                "jest".to_string(),
                "@types/jest".to_string(),
                "eslint".to_string(),
                "@typescript-eslint/parser".to_string(),
            ],
            quality_requirements: QualityRequirements {
                min_quality_score: 80.0,
                required_files: vec!["README.md".to_string(), "package.json".to_string(), "tsconfig.json".to_string()],
                required_tests: true,
                required_documentation: true,
                security_checks: vec!["npm audit".to_string()],
            },
        }
    }

    /// Create Rust basic template
    pub(crate) fn create_rust_template(&self) -> ProjectTemplate {
        ProjectTemplate {
            name: "rust_basic".to_string(),
            language: "rust".to_string(),
            framework: None,
            file_templates: HashMap::new(),
            directory_structure: vec![
                "src".to_string(),
                "tests".to_string(),
                "docs".to_string(),
                "examples".to_string(),
                ".github/workflows".to_string(),
            ],
            dependencies: vec![
                "tokio".to_string(),
                "serde".to_string(),
                "anyhow".to_string(),
            ],
            quality_requirements: QualityRequirements {
                min_quality_score: 85.0,
                required_files: vec!["README.md".to_string(), "Cargo.toml".to_string()],
                required_tests: true,
                required_documentation: true,
                security_checks: vec!["cargo audit".to_string()],
            },
        }
    }

    /// Create Rust Axum web template
    pub(crate) fn create_rust_axum_template(&self) -> ProjectTemplate {
        ProjectTemplate {
            name: "rust_axum".to_string(),
            language: "rust".to_string(),
            framework: Some("axum".to_string()),
            file_templates: HashMap::new(),
            directory_structure: vec![
                "src".to_string(),
                "src/handlers".to_string(),
                "src/models".to_string(),
                "src/services".to_string(),
                "tests".to_string(),
                "docs".to_string(),
            ],
            dependencies: vec![
                "axum".to_string(),
                "tokio".to_string(),
                "serde".to_string(),
                "sqlx".to_string(),
                "tower".to_string(),
                "tower-http".to_string(),
            ],
            quality_requirements: QualityRequirements {
                min_quality_score: 90.0,
                required_files: vec!["README.md".to_string(), "Cargo.toml".to_string()],
                required_tests: true,
                required_documentation: true,
                security_checks: vec!["cargo audit".to_string(), "cargo clippy".to_string()],
            },
        }
    }

    /// Create Go basic template
    pub(crate) fn create_go_template(&self) -> ProjectTemplate {
        ProjectTemplate {
            name: "go_basic".to_string(),
            language: "go".to_string(),
            framework: None,
            file_templates: HashMap::new(),
            directory_structure: vec![
                "cmd".to_string(),
                "internal".to_string(),
                "pkg".to_string(),
                "docs".to_string(),
                ".github/workflows".to_string(),
            ],
            dependencies: vec![],
            quality_requirements: QualityRequirements {
                min_quality_score: 80.0,
                required_files: vec!["README.md".to_string(), "go.mod".to_string()],
                required_tests: true,
                required_documentation: true,
                security_checks: vec!["gosec".to_string()],
            },
        }
    }

    /// Create Go Gin web template
    pub(crate) fn create_go_gin_template(&self) -> ProjectTemplate {
        ProjectTemplate {
            name: "go_gin".to_string(),
            language: "go".to_string(),
            framework: Some("gin".to_string()),
            file_templates: HashMap::new(),
            directory_structure: vec![
                "cmd".to_string(),
                "internal/handlers".to_string(),
                "internal/models".to_string(),
                "internal/services".to_string(),
                "pkg".to_string(),
                "docs".to_string(),
            ],
            dependencies: vec![
                "github.com/gin-gonic/gin".to_string(),
                "github.com/stretchr/testify".to_string(),
            ],
            quality_requirements: QualityRequirements {
                min_quality_score: 85.0,
                required_files: vec!["README.md".to_string(), "go.mod".to_string()],
                required_tests: true,
                required_documentation: true,
                security_checks: vec!["gosec".to_string(), "govulncheck".to_string()],
            },
        }
    }

    // Helper methods for detecting language and framework from prompts

    /// Detect programming language from prompt content
    pub(crate) fn detect_language(&self, prompt: &str) -> String {
        let prompt_lower = prompt.to_lowercase();
        
        if prompt_lower.contains("python") || prompt_lower.contains("django") || 
           prompt_lower.contains("flask") || prompt_lower.contains("fastapi") {
            return "python".to_string();
        }
        
        if prompt_lower.contains("javascript") || prompt_lower.contains("node") ||
           prompt_lower.contains("express") || prompt_lower.contains("react") {
            return "javascript".to_string();
        }
        
        if prompt_lower.contains("typescript") || prompt_lower.contains("ts") {
            return "typescript".to_string();
        }
        
        if prompt_lower.contains("rust") || prompt_lower.contains("axum") ||
           prompt_lower.contains("actix") || prompt_lower.contains("warp") {
            return "rust".to_string();
        }
        
        if prompt_lower.contains("go") || prompt_lower.contains("golang") ||
           prompt_lower.contains("gin") || prompt_lower.contains("echo") {
            return "go".to_string();
        }
        
        if prompt_lower.contains("java") || prompt_lower.contains("spring") {
            return "java".to_string();
        }
        
        // Default to Python for general prompts
        "python".to_string()
    }

    /// Detect framework from prompt content
    pub(crate) fn detect_framework(&self, prompt: &str, language: &str) -> Option<String> {
        let prompt_lower = prompt.to_lowercase();
        
        match language {
            "python" => {
                if prompt_lower.contains("flask") {
                    Some("flask".to_string())
                } else if prompt_lower.contains("fastapi") {
                    Some("fastapi".to_string())
                } else if prompt_lower.contains("django") {
                    Some("django".to_string())
                } else {
                    None
                }
            },
            "javascript" => {
                if prompt_lower.contains("express") {
                    Some("express".to_string())
                } else if prompt_lower.contains("react") {
                    Some("react".to_string())
                } else {
                    None
                }
            },
            "rust" => {
                if prompt_lower.contains("axum") {
                    Some("axum".to_string())
                } else if prompt_lower.contains("actix") {
                    Some("actix-web".to_string())
                } else {
                    None
                }
            },
            "go" => {
                if prompt_lower.contains("gin") {
                    Some("gin".to_string())
                } else if prompt_lower.contains("echo") {
                    Some("echo".to_string())
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    /// Extract complexity factors from prompt
    pub(crate) fn extract_complexity_factors(&self, prompt: &str) -> Vec<String> {
        let mut factors = vec![];
        let prompt_lower = prompt.to_lowercase();
        
        if prompt_lower.contains("api") || prompt_lower.contains("rest") || 
           prompt_lower.contains("endpoint") || prompt_lower.contains("server") {
            factors.push("api_development".to_string());
        }
        
        if prompt_lower.contains("database") || prompt_lower.contains("sql") ||
           prompt_lower.contains("postgres") || prompt_lower.contains("mysql") {
            factors.push("database_design".to_string());
        }
        
        if prompt_lower.contains("security") || prompt_lower.contains("authentication") ||
           prompt_lower.contains("authorization") || prompt_lower.contains("auth") {
            factors.push("security_implementation".to_string());
        }
        
        if prompt_lower.contains("performance") || prompt_lower.contains("optimization") ||
           prompt_lower.contains("cache") || prompt_lower.contains("benchmark") {
            factors.push("performance_optimization".to_string());
        }
        
        if prompt_lower.contains("frontend") || prompt_lower.contains("ui") ||
           prompt_lower.contains("interface") || prompt_lower.contains("web") {
            factors.push("frontend_development".to_string());
        }
        
        if prompt_lower.contains("microservice") || prompt_lower.contains("distributed") ||
           prompt_lower.contains("container") || prompt_lower.contains("docker") {
            factors.push("distributed_systems".to_string());
        }
        
        factors
    }

    /// Extract features from prompt
    pub(crate) fn extract_features(&self, prompt: &str) -> Vec<String> {
        let mut features = vec![];
        let prompt_lower = prompt.to_lowercase();
        
        if prompt_lower.contains("crud") {
            features.push("CRUD operations".to_string());
        }
        
        if prompt_lower.contains("authentication") || prompt_lower.contains("login") {
            features.push("User authentication".to_string());
        }
        
        if prompt_lower.contains("file upload") || prompt_lower.contains("upload") {
            features.push("File upload handling".to_string());
        }
        
        if prompt_lower.contains("email") || prompt_lower.contains("notification") {
            features.push("Email notifications".to_string());
        }
        
        if prompt_lower.contains("search") {
            features.push("Search functionality".to_string());
        }
        
        if prompt_lower.contains("payment") || prompt_lower.contains("stripe") {
            features.push("Payment processing".to_string());
        }
        
        features
    }

    /// Determine quality requirements based on prompt
    pub(crate) fn determine_quality_requirements(&self, prompt: &str) -> QualityRequirements {
        let prompt_lower = prompt.to_lowercase();
        
        let mut min_score: f32 = 75.0;
        let mut security_checks = vec![];
        
        // Higher standards for certain types of projects
        if prompt_lower.contains("production") || prompt_lower.contains("enterprise") {
            min_score = 90.0;
        } else if prompt_lower.contains("api") || prompt_lower.contains("server") {
            min_score = 85.0;
        }
        
        // Security requirements
        if prompt_lower.contains("security") || prompt_lower.contains("auth") {
            security_checks.push("security_scan".to_string());
            security_checks.push("vulnerability_check".to_string());
            min_score = min_score.max(85.0);
        }
        
        QualityRequirements {
            min_quality_score: min_score,
            required_files: vec!["README.md".to_string()],
            required_tests: true,
            required_documentation: true,
            security_checks,
        }
    }
}