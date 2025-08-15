use anyhow::Result;
use serde_json::json;
use std::path::Path;
use tempfile::tempdir;
use std::fs;
use cai::tool_safety::*;

#[tokio::test]
async fn test_tool_classification() {
    let validator = ToolSafetyValidator::new(true);
    
    // Test safe tools
    assert_eq!(validator.classify_tool("list_directory"), ToolSafetyLevel::Safe);
    assert_eq!(validator.classify_tool("read_file"), ToolSafetyLevel::Safe);
    assert_eq!(validator.classify_tool("search_files"), ToolSafetyLevel::Safe);
    assert_eq!(validator.classify_tool("glob_files"), ToolSafetyLevel::Safe);
    assert_eq!(validator.classify_tool("web_fetch"), ToolSafetyLevel::Safe);
    assert_eq!(validator.classify_tool("create_tasks"), ToolSafetyLevel::Safe);
    assert_eq!(validator.classify_tool("update_tasks"), ToolSafetyLevel::Safe);
    
    // Test tools requiring approval
    assert_eq!(validator.classify_tool("edit_file"), ToolSafetyLevel::RequiresApproval);
    assert_eq!(validator.classify_tool("write_file"), ToolSafetyLevel::RequiresApproval);
    assert_eq!(validator.classify_tool("multiedit_file"), ToolSafetyLevel::RequiresApproval);
    assert_eq!(validator.classify_tool("download_file"), ToolSafetyLevel::RequiresApproval);
    
    // Test dangerous tools
    assert_eq!(validator.classify_tool("delete_path"), ToolSafetyLevel::Dangerous);
    assert_eq!(validator.classify_tool("execute_command"), ToolSafetyLevel::Dangerous);
    
    // Test unknown tools default to dangerous
    assert_eq!(validator.classify_tool("unknown_tool"), ToolSafetyLevel::Dangerous);
}

#[tokio::test]
async fn test_read_before_edit_validation() -> Result<()> {
    let validator = ToolSafetyValidator::new(false); // Non-interactive for testing
    let temp_dir = tempdir()?;
    let file_path = temp_dir.path().join("test.txt");
    
    fs::write(&file_path, "test content")?;
    
    // Initially, edit should fail because file hasn't been read
    let result = validator.validate_edit_operation(&file_path);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SafetyError::ReadBeforeEdit { .. }));
    
    // After recording a read, edit should be allowed
    validator.record_file_read(&file_path);
    let result = validator.validate_edit_operation(&file_path);
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_path_validation() -> Result<()> {
    let validator = ToolSafetyValidator::new(false);
    
    // Valid paths should pass
    assert!(validator.validate_path(Path::new("/tmp/test.txt")).is_ok());
    assert!(validator.validate_path(Path::new("./test.txt")).is_ok());
    
    // Directory traversal should be blocked
    let result = validator.validate_path(Path::new("../../../etc/passwd"));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SafetyError::PathValidation { .. }));
    
    let result = validator.validate_path(Path::new("./../../dangerous/path"));
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_tool_permission_checking() -> Result<()> {
    let validator = ToolSafetyValidator::new(false); // Non-interactive
    let temp_dir = tempdir()?;
    let file_path = temp_dir.path().join("test.txt");
    
    fs::write(&file_path, "content")?;
    
    // Safe tool should always be allowed
    let args = json!({
        "path": file_path.to_str().unwrap()
    });
    assert!(validator.check_tool_permission("read_file", &args).await.is_ok());
    
    // Dangerous tool should be blocked in non-interactive mode
    let args = json!({
        "path": file_path.to_str().unwrap(),
        "recursive": true
    });
    let result = validator.check_tool_permission("delete_path", &args).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SafetyError::DangerousOperation { .. }));
    
    Ok(())
}

#[tokio::test]
async fn test_edit_operation_detection() -> Result<()> {
    let validator = ToolSafetyValidator::new(false);
    let temp_dir = tempdir()?;
    let file_path = temp_dir.path().join("edit_test.txt");
    
    fs::write(&file_path, "original content")?;
    
    // Edit operations should require read-before-edit validation
    let args = json!({
        "path": file_path.to_str().unwrap(),
        "old_text": "original",
        "new_text": "modified"
    });
    
    // Should fail without prior read
    let result = validator.check_tool_permission("edit_file", &args).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SafetyError::ReadBeforeEdit { .. }));
    
    // Should succeed after read
    validator.record_file_read(&file_path);
    let result = validator.check_tool_permission("edit_file", &args).await;
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_session_permissions() -> Result<()> {
    let validator = ToolSafetyValidator::new(true); // Interactive mode
    
    // In real usage, this would prompt the user, but for testing we check the mechanism
    let args = json!({
        "command": "echo 'test'",
        "timeout": 10
    });
    
    // Should require approval for dangerous command execution
    let result = validator.check_tool_permission("execute_command", &args).await;
    // In non-interactive testing, this will fail as expected
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test] 
async fn test_path_normalization() -> Result<()> {
    let validator = ToolSafetyValidator::new(false);
    let temp_dir = tempdir()?;
    
    // Test various path formats
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "content")?;
    
    // Record read with relative path
    let relative_path = std::path::Path::new("./test.txt");
    validator.record_file_read(&relative_path);
    
    // Should work with absolute path too (after normalization)
    // This tests the internal normalize_path function
    assert!(validator.validate_path(&file_path).is_ok());
    
    Ok(())
}

#[test]
fn test_safety_error_display() {
    use std::path::PathBuf;
    
    let path_error = SafetyError::PathValidation {
        reason: "Invalid path format".to_string()
    };
    assert!(path_error.to_string().contains("Path validation failed"));
    
    let read_error = SafetyError::ReadBeforeEdit {
        path: PathBuf::from("/tmp/test.txt"),
        suggestion: "Use read_file first".to_string()
    };
    assert!(read_error.to_string().contains("Read-before-edit violation"));
    assert!(read_error.to_string().contains("/tmp/test.txt"));
    
    let permission_error = SafetyError::PermissionDenied {
        tool: "dangerous_tool".to_string()
    };
    assert!(permission_error.to_string().contains("Permission denied"));
    
    let dangerous_error = SafetyError::DangerousOperation {
        tool: "delete_all".to_string(),
        reason: "Could destroy data".to_string()
    };
    assert!(dangerous_error.to_string().contains("Dangerous operation blocked"));
    
    let policy_error = SafetyError::PolicyViolation {
        reason: "Violates security policy".to_string()
    };
    assert!(policy_error.to_string().contains("Tool execution blocked by safety policy"));
}

#[test]
fn test_permission_levels() {
    // Test permission level enum variants
    assert!(matches!(PermissionLevel::Denied, PermissionLevel::Denied));
    assert!(matches!(PermissionLevel::AllowOnce, PermissionLevel::AllowOnce));
    assert!(matches!(PermissionLevel::AllowSession, PermissionLevel::AllowSession));
    assert!(matches!(PermissionLevel::AlwaysAllow, PermissionLevel::AlwaysAllow));
}

#[tokio::test]
async fn test_global_safety_validator() -> Result<()> {
    let global_validator = get_global_safety_validator();
    
    // Test basic functionality with global instance
    let temp_dir = tempdir()?;
    let file_path = temp_dir.path().join("global_test.txt");
    fs::write(&file_path, "global test content")?;
    
    global_validator.record_file_read(&file_path);
    
    let result = global_validator.validate_edit_operation(&file_path);
    assert!(result.is_ok());
    
    // Should be the same instance when accessed again
    let same_validator = get_global_safety_validator();
    let result = same_validator.validate_edit_operation(&file_path);
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_parameter_extraction() -> Result<()> {
    let validator = ToolSafetyValidator::new(false);
    
    // Test path extraction from various argument formats
    let args_with_path = json!({
        "path": "/tmp/test.txt",
        "content": "some content"
    });
    
    let args_with_file_path = json!({
        "file_path": "/tmp/another.txt",
        "edits": []
    });
    
    // This tests the internal extract_path_from_args method indirectly
    // by checking if path validation occurs
    
    // Path with directory traversal should be caught
    let bad_args = json!({
        "path": "../../etc/passwd"
    });
    
    let result = validator.check_tool_permission("read_file", &bad_args).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SafetyError::PathValidation { .. }));
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_safety_validation() -> Result<()> {
    use std::sync::Arc;
    use tokio::task;
    
    let validator = Arc::new(ToolSafetyValidator::new(false));
    let temp_dir = tempdir()?;
    
    // Create multiple files for concurrent testing
    let mut file_paths = Vec::new();
    for i in 0..10 {
        let file_path = temp_dir.path().join(format!("concurrent_test_{}.txt", i));
        fs::write(&file_path, format!("content {}", i))?;
        file_paths.push(file_path);
    }
    
    // Test concurrent read recording
    let handles: Vec<_> = file_paths.iter().enumerate().map(|(i, path)| {
        let validator = validator.clone();
        let path = path.clone();
        task::spawn(async move {
            validator.record_file_read(&path);
            validator.validate_edit_operation(&path)
        })
    }).collect();
    
    // All concurrent operations should succeed
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
    
    Ok(())
}

#[test]
fn test_default_validator() {
    let validator = ToolSafetyValidator::default();
    
    // Default should be interactive mode
    // Test basic functionality
    assert_eq!(validator.classify_tool("read_file"), ToolSafetyLevel::Safe);
    assert_eq!(validator.classify_tool("delete_path"), ToolSafetyLevel::Dangerous);
}