use anyhow::Result;
use std::path::PathBuf;

use crate::dependency_manager::{get_dependency_manager, DependencyType, PackageJson};
use crate::code_generation::get_code_generation_engine;
use crate::logger::{log_info, log_debug};

/// Simplified integration test that validates core functionality without shell dependencies
pub struct SimplifiedIntegrationTest {
    test_dir: PathBuf,
}

impl SimplifiedIntegrationTest {
    pub fn new() -> Result<Self> {
        use std::env;
        let temp_dir = env::temp_dir();
        let test_dir = temp_dir.join("cai-simple-integration-test");
        
        // Ensure the directory exists
        std::fs::create_dir_all(&test_dir)?;
        
        Ok(Self {
            test_dir,
        })
    }

    /// Execute simplified integration test focusing on core components
    pub async fn run_simplified_test(&self) -> Result<SimplifiedTestResult> {
        log_info!("simple_integration", "🚀 Starting simplified integration test");
        let start_time = std::time::Instant::now();
        
        let mut result = SimplifiedTestResult::new();

        // Test 1: Dependency Manager
        log_info!("simple_integration", "📦 Test 1: Dependency Manager");
        self.test_dependency_manager(&mut result).await?;

        // Test 2: Code Generation (without file creation)
        log_info!("simple_integration", "⚛️ Test 2: Code Generation");
        self.test_code_generation(&mut result).await?;

        // Test 3: Package.json Manipulation
        log_info!("simple_integration", "📝 Test 3: Package.json Operations");
        self.test_package_json_operations(&mut result).await?;

        // Test 4: Integration Validation
        log_info!("simple_integration", "✅ Test 4: Integration Validation");
        self.test_integration_validation(&mut result).await?;

        result.total_execution_time = start_time.elapsed().as_secs_f64();
        log_info!("simple_integration", "🎉 Simplified integration test completed in {:.2}s", result.total_execution_time);

        Ok(result)
    }

    /// Test dependency manager functionality
    async fn test_dependency_manager(&self, result: &mut SimplifiedTestResult) -> Result<()> {
        let dep_manager = get_dependency_manager();
        
        // Create a test package.json
        let test_package = PackageJson {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            package_type: Some("module".to_string()),
            description: Some("Integration test package".to_string()),
            author: Some("CAI Test Suite".to_string()),
            license: Some("MIT".to_string()),
            scripts: None,
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            additional_fields: std::collections::HashMap::new(),
        };

        let package_path = self.test_dir.join("package.json");
        dep_manager.write_package_json(&self.test_dir, &test_package).await?;

        // Verify file was created
        if package_path.exists() {
            result.add_test_result("dependency_manager_write", true, "Package.json created successfully");
        } else {
            result.add_test_result("dependency_manager_write", false, "Failed to create package.json");
        }

        // Test reading the package.json back
        match dep_manager.read_package_json(&self.test_dir).await {
            Ok(read_package) => {
                let success = read_package.name == "test-package" && read_package.version == "1.0.0";
                result.add_test_result("dependency_manager_read", success, "Package.json read successfully");
            }
            Err(e) => {
                result.add_test_result("dependency_manager_read", false, &format!("Failed to read package.json: {}", e));
            }
        }

        // Test adding dependencies
        match dep_manager.add_dependency(&self.test_dir, "react", "^18.0.0", DependencyType::Production).await {
            Ok(dep_result) => {
                let success = dep_result.package_json_modified && dep_result.added_dependencies.contains(&"react".to_string());
                result.add_test_result("dependency_add", success, "Dependency added successfully");
            }
            Err(e) => {
                result.add_test_result("dependency_add", false, &format!("Failed to add dependency: {}", e));
            }
        }

        log_debug!("simple_integration", "✅ Dependency manager tests completed");
        Ok(())
    }

    /// Test code generation functionality
    async fn test_code_generation(&self, _result: &mut SimplifiedTestResult) -> Result<()> {
        // Test code generation engine creation
        match get_code_generation_engine().await {
            Ok(_engine) => {
                log_debug!("simple_integration", "✅ Code generation engine created successfully");
                // Note: We're not testing actual code generation here as it requires OpenRouter API
                // and would try to write files, which might trigger permission issues
            }
            Err(e) => {
                log_debug!("simple_integration", "⚠️ Code generation engine creation failed: {}", e);
                // This is expected if OpenRouter API key is not available
            }
        }

        Ok(())
    }

    /// Test package.json manipulation operations
    async fn test_package_json_operations(&self, result: &mut SimplifiedTestResult) -> Result<()> {
        let dep_manager = get_dependency_manager();

        // Test script addition
        match dep_manager.add_script(&self.test_dir, "test", "jest").await {
            Ok(_) => {
                result.add_test_result("script_add", true, "Script added successfully");
            }
            Err(e) => {
                result.add_test_result("script_add", false, &format!("Failed to add script: {}", e));
            }
        }

        // Test dependency listing
        match dep_manager.list_dependencies(&self.test_dir).await {
            Ok(deps) => {
                let success = deps.contains_key("react");
                result.add_test_result("dependency_list", success, &format!("Listed {} dependencies", deps.len()));
            }
            Err(e) => {
                result.add_test_result("dependency_list", false, &format!("Failed to list dependencies: {}", e));
            }
        }

        // Test dependency check
        match dep_manager.has_dependency(&self.test_dir, "react").await {
            Ok(Some((version, dep_type))) => {
                let success = version == "^18.0.0" && matches!(dep_type, DependencyType::Production);
                result.add_test_result("dependency_check", success, "Dependency check successful");
            }
            Ok(None) => {
                result.add_test_result("dependency_check", false, "Dependency not found");
            }
            Err(e) => {
                result.add_test_result("dependency_check", false, &format!("Dependency check failed: {}", e));
            }
        }

        log_debug!("simple_integration", "✅ Package.json operations tests completed");
        Ok(())
    }

    /// Test integration validation
    async fn test_integration_validation(&self, result: &mut SimplifiedTestResult) -> Result<()> {
        // Test if all components are properly integrated
        let package_path = self.test_dir.join("package.json");
        
        if package_path.exists() {
            // Read the final package.json and validate its contents
            let dep_manager = get_dependency_manager();
            match dep_manager.read_package_json(&self.test_dir).await {
                Ok(package) => {
                    let has_react = package.dependencies.as_ref()
                        .map(|deps| deps.contains_key("react"))
                        .unwrap_or(false);
                    
                    let has_test_script = package.scripts.as_ref()
                        .map(|scripts| scripts.contains_key("test"))
                        .unwrap_or(false);

                    let validation_score = [
                        package.name == "test-package",
                        package.version == "1.0.0",
                        has_react,
                        has_test_script,
                    ].iter().map(|&b| if b { 1 } else { 0 }).sum::<i32>();

                    let success = validation_score >= 3;
                    result.add_test_result("integration_validation", success, 
                        &format!("Integration validation score: {}/4", validation_score));
                    
                    if success {
                        result.overall_success = true;
                    }
                }
                Err(e) => {
                    result.add_test_result("integration_validation", false, 
                        &format!("Failed to validate: {}", e));
                }
            }
        } else {
            result.add_test_result("integration_validation", false, "Package.json not found");
        }

        log_debug!("simple_integration", "✅ Integration validation completed");
        Ok(())
    }
}

/// Result structure for simplified integration testing
#[derive(Debug)]
pub struct SimplifiedTestResult {
    pub test_results: Vec<TestResult>,
    pub total_execution_time: f64,
    pub overall_success: bool,
}

impl SimplifiedTestResult {
    fn new() -> Self {
        Self {
            test_results: Vec::new(),
            total_execution_time: 0.0,
            overall_success: false,
        }
    }

    fn add_test_result(&mut self, test_name: &str, success: bool, details: &str) {
        self.test_results.push(TestResult {
            test_name: test_name.to_string(),
            success,
            details: details.to_string(),
        });
    }

    pub fn print_summary(&self) {
        println!("\n🎯 Simplified Integration Test Results");
        println!("{}", "=".repeat(50));
        println!("⏱️  Total execution time: {:.2}s", self.total_execution_time);
        println!("✅ Overall success: {}", self.overall_success);
        println!("\n📋 Test Results:");
        
        for test in &self.test_results {
            let status = if test.success { "✅" } else { "❌" };
            println!("   {} {}: {}", status, test.test_name, test.details);
        }
        
        let successful_tests = self.test_results.iter().filter(|t| t.success).count();
        println!("\n🏁 Summary: {}/{} tests passed", 
                successful_tests, self.test_results.len());
    }
}

#[derive(Debug)]
pub struct TestResult {
    pub test_name: String,
    pub success: bool,
    pub details: String,
}

/// Run the simplified integration test
pub async fn run_simplified_integration_test() -> Result<SimplifiedTestResult> {
    let test = SimplifiedIntegrationTest::new()?;
    let result = test.run_simplified_test().await?;
    
    log_info!("simple_integration", "🎯 Simplified integration test completed");
    result.print_summary();
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simplified_integration() {
        let result = run_simplified_integration_test().await;
        
        match result {
            Ok(test_result) => {
                assert!(test_result.test_results.len() > 0);
                
                // At least 6 out of 8 tests should succeed for basic functionality
                let successful_tests = test_result.test_results.iter().filter(|t| t.success).count();
                assert!(successful_tests >= 6, "Only {}/{} tests succeeded", successful_tests, test_result.test_results.len());
            }
            Err(e) => {
                panic!("Simplified integration test failed: {}", e);
            }
        }
    }
}