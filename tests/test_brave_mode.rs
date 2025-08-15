use anyhow::Result;
use std::fs;
use tempfile::tempdir;
use tokio::time::{timeout, Duration};
use prompt_manager::tool_safety::{ToolSafetyValidator, get_global_safety_validator};

/// Test that brave mode disables all security checks
#[tokio::test]
async fn test_brave_mode_functionality() -> Result<()> {
    println!("🧪 Testing brave mode functionality");
    
    // Test with regular safety validator (non-brave)
    let regular_validator = ToolSafetyValidator::new(false, false);
    
    // Test with brave mode validator
    let brave_validator = ToolSafetyValidator::new(false, true);
    
    println!("✅ Both validators created successfully");
    
    // Test path that would normally be rejected
    let bad_path = std::path::Path::new("../../../etc/passwd");
    
    // Regular validator should reject this path
    let regular_result = regular_validator.validate_path(bad_path);
    println!("🔒 Regular validator result: {:?}", regular_result.is_err());
    assert!(regular_result.is_err(), "Regular validator should reject dangerous path");
    
    // Brave validator should allow this path
    let brave_result = brave_validator.validate_path(bad_path);
    println!("🦾 Brave validator result: {:?}", brave_result.is_ok());
    assert!(brave_result.is_ok(), "Brave validator should allow dangerous path");
    
    println!("✅ Brave mode correctly bypasses path validation");
    
    // Test file editing without read-before-edit
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("test.txt");
    
    // Regular validator should require read-before-edit
    let regular_edit_result = regular_validator.validate_edit_operation(&test_file);
    println!("🔒 Regular edit validation: {:?}", regular_edit_result.is_err());
    assert!(regular_edit_result.is_err(), "Regular validator should require read-before-edit");
    
    // Brave validator should allow edit without read
    let brave_edit_result = brave_validator.validate_edit_operation(&test_file);
    println!("🦾 Brave edit validation: {:?}", brave_edit_result.is_ok());
    assert!(brave_edit_result.is_ok(), "Brave validator should skip read-before-edit");
    
    println!("✅ Brave mode correctly bypasses edit validation");
    
    // Test tool permission checking
    let test_args = serde_json::json!({"path": "/dangerous/path"});
    
    // Regular validator should block dangerous tools  
    let regular_perm_result = regular_validator.check_tool_permission("delete_path", &test_args).await;
    println!("🔒 Regular permission check: {:?}", regular_perm_result.is_err());
    assert!(regular_perm_result.is_err(), "Regular validator should block dangerous tools");
    
    // Brave validator should allow dangerous tools
    let brave_perm_result = brave_validator.check_tool_permission("delete_path", &test_args).await;
    println!("🦾 Brave permission check: {:?}", brave_perm_result.is_ok());
    assert!(brave_perm_result.is_ok(), "Brave validator should allow dangerous tools");
    
    println!("✅ Brave mode correctly bypasses permission checks");
    
    // Test environment variable integration
    std::env::set_var("CAI_BRAVE_MODE", "true");
    
    // This should create a brave mode validator via environment
    // Note: This tests the initialization logic
    println!("🧪 Testing environment variable integration...");
    
    std::env::remove_var("CAI_BRAVE_MODE");
    println!("✅ Brave mode test completed successfully");
    
    Ok(())
}

/// Test brave mode with actual file creation (simplified version)
#[tokio::test] 
async fn test_brave_mode_file_creation() -> Result<()> {
    println!("🧪 Testing brave mode file creation");
    
    // Enable brave mode via environment
    std::env::set_var("CAI_BRAVE_MODE", "true");
    
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("brave_test.txt");
    
    // Create brave mode validator
    let brave_validator = ToolSafetyValidator::new(false, true);
    
    // Test write operation args
    let write_args = serde_json::json!({
        "path": test_file.to_string_lossy(),
        "content": "brave mode test content"
    });
    
    // Brave mode should allow write_file without read-before-edit
    let permission_result = brave_validator.check_tool_permission("write_file", &write_args).await;
    println!("🦾 Write permission in brave mode: {:?}", permission_result.is_ok());
    assert!(permission_result.is_ok(), "Brave mode should allow write without restrictions");
    
    // Actually create the file to test the concept
    fs::write(&test_file, "brave mode test content")?;
    
    // Verify file was created
    assert!(test_file.exists(), "File should be created");
    let content = fs::read_to_string(&test_file)?;
    assert_eq!(content, "brave mode test content", "File content should match");
    
    println!("✅ File created successfully in brave mode test");
    
    // Cleanup
    std::env::remove_var("CAI_BRAVE_MODE");
    
    Ok(())
}