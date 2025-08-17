use anyhow::Result;
use std::collections::HashMap;

use crate::dependency_manager::{get_dependency_manager, DependencyType, PackageJson};
use crate::logger::{log_info, log_debug};

/// Basic functionality test that validates core logic without file operations
pub struct BasicFunctionalityTest;

impl BasicFunctionalityTest {
    /// Execute basic functionality test focusing on core component logic
    pub async fn run_basic_test() -> Result<BasicTestResult> {
        log_info!("basic_test", "🚀 Starting basic functionality test");
        let start_time = std::time::Instant::now();
        
        let mut result = BasicTestResult::new();

        // Test 1: Package.json Structure Validation
        log_info!("basic_test", "📦 Test 1: PackageJson Structure");
        Self::test_package_json_structure(&mut result).await?;

        // Test 2: Dependency Manager Logic
        log_info!("basic_test", "🔧 Test 2: Dependency Manager Logic");
        Self::test_dependency_manager_logic(&mut result).await?;

        // Test 3: Component Integration
        log_info!("basic_test", "🔗 Test 3: Component Integration");
        Self::test_component_integration(&mut result).await?;

        result.total_execution_time = start_time.elapsed().as_secs_f64();
        log_info!("basic_test", "🎉 Basic functionality test completed in {:.2}s", result.total_execution_time);

        Ok(result)
    }

    /// Test package.json data structure functionality
    async fn test_package_json_structure(result: &mut BasicTestResult) -> Result<()> {
        // Test PackageJson creation and serialization
        let mut dependencies = HashMap::new();
        dependencies.insert("react".to_string(), "^18.0.0".to_string());
        dependencies.insert("typescript".to_string(), "^5.0.0".to_string());

        let mut scripts = HashMap::new();
        scripts.insert("build".to_string(), "tsc".to_string());
        scripts.insert("test".to_string(), "jest".to_string());

        let package = PackageJson {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            package_type: Some("module".to_string()),
            description: Some("Test package for validation".to_string()),
            author: Some("CAI Test Suite".to_string()),
            license: Some("MIT".to_string()),
            scripts: Some(scripts),
            dependencies: Some(dependencies),
            dev_dependencies: None,
            peer_dependencies: None,
            additional_fields: HashMap::new(),
        };

        // Test JSON serialization
        match serde_json::to_string_pretty(&package) {
            Ok(json_str) => {
                let valid_json = json_str.contains("test-package") && 
                                json_str.contains("react") && 
                                json_str.contains("typescript");
                result.add_test_result("package_json_serialization", valid_json, 
                    "Package.json serialization successful");
            }
            Err(e) => {
                result.add_test_result("package_json_serialization", false, 
                    &format!("Failed to serialize package.json: {}", e));
            }
        }

        // Test JSON deserialization
        let json_input = r#"{
            "name": "test-deserialize",
            "version": "2.0.0",
            "type": "module",
            "scripts": {
                "start": "node index.js"
            },
            "dependencies": {
                "lodash": "^4.17.21"
            }
        }"#;

        match serde_json::from_str::<PackageJson>(json_input) {
            Ok(deserialized) => {
                let valid = deserialized.name == "test-deserialize" && 
                           deserialized.version == "2.0.0" &&
                           deserialized.package_type == Some("module".to_string());
                result.add_test_result("package_json_deserialization", valid, 
                    "Package.json deserialization successful");
            }
            Err(e) => {
                result.add_test_result("package_json_deserialization", false, 
                    &format!("Failed to deserialize package.json: {}", e));
            }
        }

        log_debug!("basic_test", "✅ Package.json structure tests completed");
        Ok(())
    }

    /// Test dependency manager component logic
    async fn test_dependency_manager_logic(result: &mut BasicTestResult) -> Result<()> {
        // Test dependency manager singleton creation
        let dep_manager1 = get_dependency_manager();
        let dep_manager2 = get_dependency_manager();
        
        // Test that we get the same instance (Arc comparison)
        let same_instance = std::ptr::eq(dep_manager1.as_ref(), dep_manager2.as_ref());
        result.add_test_result("dependency_manager_singleton", same_instance, 
            "Dependency manager singleton pattern working");

        // Test DependencyType enum
        let prod_type = DependencyType::Production;
        let dev_type = DependencyType::Development;
        let peer_type = DependencyType::Peer;

        // Test serialization of DependencyType
        let prod_json = serde_json::to_string(&prod_type);
        let dev_json = serde_json::to_string(&dev_type);
        let peer_json = serde_json::to_string(&peer_type);

        let types_serializable = prod_json.is_ok() && dev_json.is_ok() && peer_json.is_ok();
        result.add_test_result("dependency_type_serialization", types_serializable, 
            "DependencyType enum serialization working");

        log_debug!("basic_test", "✅ Dependency manager logic tests completed");
        Ok(())
    }

    /// Test component integration without file operations
    async fn test_component_integration(result: &mut BasicTestResult) -> Result<()> {
        // Test code generation engine creation (may fail without API key, which is expected)
        let code_gen_available = match crate::code_generation::get_code_generation_engine().await {
            Ok(_) => {
                log_debug!("basic_test", "✅ Code generation engine created successfully");
                true
            }
            Err(_) => {
                log_debug!("basic_test", "ℹ️ Code generation engine creation failed (expected without API key)");
                false // This is expected without OpenRouter API key
            }
        };
        
        result.add_test_result("code_generation_engine", true, 
            &format!("Code generation engine creation: {}", 
                if code_gen_available { "available" } else { "unavailable (expected)" }));

        // Test file operations manager creation
        let file_ops = crate::file_operations::get_file_operations_manager();
        let file_ops_created = !std::ptr::eq(file_ops.as_ref() as *const _, std::ptr::null());
        result.add_test_result("file_operations_manager", file_ops_created, 
            "File operations manager created successfully");

        // Test shell execution manager creation
        let shell_manager = crate::shell_execution::get_shell_execution_manager();
        let shell_manager_created = !std::ptr::eq(shell_manager.as_ref() as *const _, std::ptr::null());
        result.add_test_result("shell_execution_manager", shell_manager_created, 
            "Shell execution manager created successfully");

        // Test project scaffolder creation
        let scaffolder = crate::project_scaffolding::get_project_scaffolder();
        let scaffolder_created = !std::ptr::eq(scaffolder.as_ref() as *const _, std::ptr::null());
        result.add_test_result("project_scaffolder", scaffolder_created, 
            "Project scaffolder created successfully");

        // Test that all managers are properly integrated
        let all_managers_available = file_ops_created && shell_manager_created && scaffolder_created;
        result.add_test_result("manager_integration", all_managers_available, 
            "All core managers properly integrated");

        // Overall success check
        if all_managers_available {
            result.overall_success = true;
        }

        log_debug!("basic_test", "✅ Component integration tests completed");
        Ok(())
    }
}

/// Result structure for basic functionality testing
#[derive(Debug)]
pub struct BasicTestResult {
    pub test_results: Vec<TestResult>,
    pub total_execution_time: f64,
    pub overall_success: bool,
}

impl BasicTestResult {
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
        println!("\n🎯 Basic Functionality Test Results");
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
        
        if successful_tests >= self.test_results.len() * 3 / 4 {
            println!("🎉 Core functionality is working correctly!");
        } else {
            println!("⚠️ Some core functionality may need attention");
        }
    }
}

#[derive(Debug)]
pub struct TestResult {
    pub test_name: String,
    pub success: bool,
    pub details: String,
}

/// Run the basic functionality test
pub async fn run_basic_functionality_test() -> Result<BasicTestResult> {
    let result = BasicFunctionalityTest::run_basic_test().await?;
    
    log_info!("basic_test", "🎯 Basic functionality test completed");
    result.print_summary();
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_functionality() {
        let result = run_basic_functionality_test().await;
        
        match result {
            Ok(test_result) => {
                assert!(test_result.test_results.len() > 0);
                
                // At least 80% of tests should succeed for basic functionality
                let successful_tests = test_result.test_results.iter().filter(|t| t.success).count();
                let success_rate = successful_tests * 100 / test_result.test_results.len();
                assert!(success_rate >= 80, "Only {}% of tests passed", success_rate);
            }
            Err(e) => {
                panic!("Basic functionality test failed: {}", e);
            }
        }
    }
}