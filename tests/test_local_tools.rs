use anyhow::Result;
use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;
use cai::local_tools::*;

#[tokio::test]
async fn test_list_local_tools() {
    let tools = list_local_tools();
    
    // Verify all expected tools are present
    assert!(tools.contains(&"list_directory".to_string()));
    assert!(tools.contains(&"read_file".to_string()));
    assert!(tools.contains(&"write_file".to_string()));
    assert!(tools.contains(&"edit_file".to_string()));
    assert!(tools.contains(&"delete_path".to_string()));
    assert!(tools.contains(&"search_files".to_string()));
    assert!(tools.contains(&"execute_command".to_string()));
    assert!(tools.contains(&"glob_files".to_string()));
    assert!(tools.contains(&"web_fetch".to_string()));
    assert!(tools.contains(&"create_tasks".to_string()));
    assert!(tools.contains(&"update_tasks".to_string()));
    assert!(tools.contains(&"download_file".to_string()));
    assert!(tools.contains(&"multiedit_file".to_string()));
    assert!(tools.contains(&"read_many_files".to_string()));
    assert!(tools.contains(&"diff_files".to_string()));
    assert!(tools.contains(&"batch_file_search".to_string()));
    
    // Should have all the tools we added
    assert!(tools.len() >= 16);
}

#[tokio::test]
async fn test_list_directory() -> Result<()> {
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();
    
    // Create test files
    fs::write(temp_path.join("file1.txt"), "content1")?;
    fs::write(temp_path.join("file2.txt"), "content2")?;
    fs::create_dir(temp_path.join("subdir"))?;
    
    let args = json!({
        "path": temp_path.to_str().unwrap()
    });
    
    let result = execute_local_tool("list_directory", args)?;
    
    assert_eq!(result["ok"], true);
    let entries = result["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    
    // Check that we have the expected files/directories
    let names: Vec<&str> = entries.iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"file1.txt"));
    assert!(names.contains(&"file2.txt"));
    assert!(names.contains(&"subdir"));
    
    Ok(())
}

#[tokio::test]
async fn test_read_file() -> Result<()> {
    let temp_dir = tempdir()?;
    let file_path = temp_dir.path().join("test.txt");
    
    let content = "Line 1\nLine 2\nLine 3\n";
    fs::write(&file_path, content)?;
    
    let args = json!({
        "path": file_path.to_str().unwrap()
    });
    
    let result = execute_local_tool("read_file", args)?;
    
    assert_eq!(result["ok"], true);
    assert_eq!(result["content"], content);
    assert_eq!(result["total_lines"], 4); // Including empty line at end
    
    Ok(())
}

#[tokio::test]
async fn test_read_file_with_range() -> Result<()> {
    let temp_dir = tempdir()?;
    let file_path = temp_dir.path().join("test.txt");
    
    let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
    fs::write(&file_path, content)?;
    
    let args = json!({
        "path": file_path.to_str().unwrap(),
        "start_line": 2,
        "end_line": 4
    });
    
    let result = execute_local_tool("read_file", args)?;
    
    assert_eq!(result["ok"], true);
    assert_eq!(result["content"], "Line 2\nLine 3\nLine 4");
    assert_eq!(result["lines_shown"], [2, 4]);
    
    Ok(())
}

#[tokio::test]
async fn test_write_file() -> Result<()> {
    let temp_dir = tempdir()?;
    let file_path = temp_dir.path().join("new_file.txt");
    
    let args = json!({
        "path": file_path.to_str().unwrap(),
        "content": "Hello, World!",
        "overwrite": true
    });
    
    let result = execute_local_tool("write_file", args)?;
    
    assert_eq!(result["ok"], true);
    
    // Verify file was created
    let written_content = fs::read_to_string(&file_path)?;
    assert_eq!(written_content, "Hello, World!");
    
    Ok(())
}

#[tokio::test]
async fn test_edit_file() -> Result<()> {
    let temp_dir = tempdir()?;
    let file_path = temp_dir.path().join("edit_test.txt");
    
    let original_content = "Hello, world!\nThis is a test.\nGoodbye!";
    fs::write(&file_path, original_content)?;
    
    // First read the file to satisfy read-before-edit requirement
    let read_args = json!({
        "path": file_path.to_str().unwrap()
    });
    execute_local_tool("read_file", read_args)?;
    
    let args = json!({
        "path": file_path.to_str().unwrap(),
        "old_text": "world",
        "new_text": "Rust"
    });
    
    let result = execute_local_tool("edit_file", args)?;
    
    assert_eq!(result["ok"], true);
    assert_eq!(result["replacements"], 1);
    
    // Verify edit was made
    let edited_content = fs::read_to_string(&file_path)?;
    assert!(edited_content.contains("Hello, Rust!"));
    
    Ok(())
}

#[tokio::test]
async fn test_search_files() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create test files with content
    fs::write(temp_dir.path().join("file1.rs"), "fn main() {\n    println!(\"Hello\");\n}")?;
    fs::write(temp_dir.path().join("file2.rs"), "struct Test {\n    value: i32,\n}")?;
    fs::write(temp_dir.path().join("file3.txt"), "This is a text file")?;
    
    let args = json!({
        "pattern": "println",
        "directory": temp_dir.path().to_str().unwrap(),
        "case_sensitive": false,
        "max_results": 100,
        "pattern_type": "substring",
        "file_pattern": "*.rs"
    });
    
    let result = execute_local_tool("search_files", args)?;
    
    assert_eq!(result["ok"], true);
    let matches = result["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1); // Only one file contains "println"
    
    assert!(matches[0]["file"].as_str().unwrap().contains("file1.rs"));
    assert_eq!(matches[0]["line"], 2);
    
    Ok(())
}

#[tokio::test]
async fn test_glob_files() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create test files
    fs::write(temp_dir.path().join("test1.rs"), "content")?;
    fs::write(temp_dir.path().join("test2.rs"), "content")?;
    fs::write(temp_dir.path().join("readme.txt"), "content")?;
    fs::create_dir(temp_dir.path().join("subdir"))?;
    
    let args = json!({
        "pattern": "*.rs",
        "directory": temp_dir.path().to_str().unwrap()
    });
    
    let result = execute_local_tool("glob_files", args)?;
    
    assert_eq!(result["ok"], true);
    let matches = result["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 2);
    
    // Should find both .rs files
    let file_names: Vec<&str> = matches.iter()
        .map(|m| m["path"].as_str().unwrap())
        .collect();
    assert!(file_names.iter().any(|name| name.contains("test1.rs")));
    assert!(file_names.iter().any(|name| name.contains("test2.rs")));
    
    Ok(())
}

#[tokio::test]
async fn test_read_many_files() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create test files
    fs::write(temp_dir.path().join("file1.txt"), "Content of file 1")?;
    fs::write(temp_dir.path().join("file2.txt"), "Content of file 2")?;
    fs::write(temp_dir.path().join("file3.txt"), "Content of file 3")?;
    
    let args = json!({
        "file_paths": [
            temp_dir.path().join("file1.txt").to_str().unwrap(),
            temp_dir.path().join("file2.txt").to_str().unwrap(),
            temp_dir.path().join("file3.txt").to_str().unwrap(),
        ],
        "include_line_numbers": false
    });
    
    let result = execute_local_tool("read_many_files", args)?;
    
    assert_eq!(result["ok"], true);
    assert_eq!(result["files_read"], 3);
    assert_eq!(result["files_failed"], 0);
    
    let results = result["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    
    // Check content
    assert!(results.iter().any(|r| r["content"].as_str().unwrap() == "Content of file 1"));
    assert!(results.iter().any(|r| r["content"].as_str().unwrap() == "Content of file 2"));
    assert!(results.iter().any(|r| r["content"].as_str().unwrap() == "Content of file 3"));
    
    Ok(())
}

#[tokio::test]
async fn test_diff_files() -> Result<()> {
    let temp_dir = tempdir()?;
    
    let file1_path = temp_dir.path().join("file1.txt");
    let file2_path = temp_dir.path().join("file2.txt");
    
    fs::write(&file1_path, "Line 1\nLine 2\nLine 3")?;
    fs::write(&file2_path, "Line 1\nLine 2 modified\nLine 3\nLine 4")?;
    
    let args = json!({
        "file1": file1_path.to_str().unwrap(),
        "file2": file2_path.to_str().unwrap(),
        "diff_type": "unified",
        "context_lines": 1
    });
    
    let result = execute_local_tool("diff_files", args)?;
    
    assert_eq!(result["ok"], true);
    assert_eq!(result["identical"], false);
    assert_eq!(result["file1_lines"], 3);
    assert_eq!(result["file2_lines"], 4);
    assert!(result["changes"].as_u64().unwrap() > 0);
    assert!(result["additions"].as_u64().unwrap() > 0);
    
    let diff_output = result["diff_output"].as_str().unwrap();
    assert!(diff_output.contains("Line 2"));
    assert!(diff_output.contains("modified"));
    
    Ok(())
}

#[tokio::test]
async fn test_batch_file_search() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create directory structure
    fs::create_dir(temp_dir.path().join("src"))?;
    fs::create_dir(temp_dir.path().join("docs"))?;
    
    // Create test files
    fs::write(temp_dir.path().join("src/main.rs"), "fn main() {\n    println!(\"Hello\");\n}")?;
    fs::write(temp_dir.path().join("src/lib.rs"), "pub fn hello() {\n    println!(\"World\");\n}")?;
    fs::write(temp_dir.path().join("docs/readme.md"), "# Project\n\nHello world!")?;
    fs::write(temp_dir.path().join("config.toml"), "[package]\nname = \"hello\"")?;
    
    let args = json!({
        "pattern": "hello",
        "directories": [temp_dir.path().to_str().unwrap()],
        "file_extensions": ["rs", "md", "toml"],
        "case_sensitive": false,
        "include_content": true,
        "max_results": 100
    });
    
    let result = execute_local_tool("batch_file_search", args)?;
    
    assert_eq!(result["ok"], true);
    assert!(result["files_scanned"].as_u64().unwrap() >= 4);
    assert!(result["total_matches"].as_u64().unwrap() >= 2); // "hello" appears in multiple files
    
    let results = result["results"].as_array().unwrap();
    assert!(results.len() >= 2);
    
    Ok(())
}

#[tokio::test]
async fn test_multiedit_file() -> Result<()> {
    let temp_dir = tempdir()?;
    let file_path = temp_dir.path().join("multiedit_test.txt");
    
    let original_content = "Hello, world!\nThis is line 2.\nThis is line 3.";
    fs::write(&file_path, original_content)?;
    
    // Read file first for safety validation
    let read_args = json!({
        "path": file_path.to_str().unwrap()
    });
    execute_local_tool("read_file", read_args)?;
    
    let args = json!({
        "file_path": file_path.to_str().unwrap(),
        "edits": [
            {
                "old_string": "world",
                "new_string": "Rust"
            },
            {
                "old_string": "line 2",
                "new_string": "the second line"
            }
        ]
    });
    
    let result = execute_local_tool("multiedit_file", args)?;
    
    assert_eq!(result["ok"], true);
    assert_eq!(result["edits_applied"], 2);
    
    // Verify changes were made
    let edited_content = fs::read_to_string(&file_path)?;
    assert!(edited_content.contains("Hello, Rust!"));
    assert!(edited_content.contains("This is the second line."));
    
    Ok(())
}

#[tokio::test]
async fn test_create_and_update_tasks() -> Result<()> {
    let tasks = vec![
        json!({
            "id": "task1",
            "description": "First task",
            "status": "pending"
        }),
        json!({
            "id": "task2",
            "description": "Second task", 
            "status": "pending"
        })
    ];
    
    let create_args = json!({
        "user_query": "Create test tasks",
        "tasks": tasks
    });
    
    let result = execute_local_tool("create_tasks", create_args)?;
    
    assert_eq!(result["ok"], true);
    assert_eq!(result["task_list"]["tasks"].as_array().unwrap().len(), 2);
    
    // Now update tasks
    let update_args = json!({
        "task_updates": [
            {
                "id": "task1",
                "status": "completed",
                "notes": "Task completed successfully"
            }
        ]
    });
    
    let update_result = execute_local_tool("update_tasks", update_args)?;
    
    assert_eq!(update_result["ok"], true);
    assert_eq!(update_result["updates_made"].as_array().unwrap().len(), 1);
    
    Ok(())
}

#[tokio::test]
async fn test_error_conditions() -> Result<()> {
    // Test unknown tool
    let result = execute_local_tool("unknown_tool", json!({}));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown local tool"));
    
    // Test missing required arguments
    let result = execute_local_tool("read_file", json!({}));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("missing 'path'"));
    
    // Test reading non-existent file
    let result = execute_local_tool("read_file", json!({"path": "/nonexistent/file.txt"}));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("file not found"));
    
    Ok(())
}

#[tokio::test]
async fn test_safety_validation() -> Result<()> {
    let temp_dir = tempdir()?;
    let file_path = temp_dir.path().join("safety_test.txt");
    
    fs::write(&file_path, "original content")?;
    
    // Try to edit without reading first - should fail
    let args = json!({
        "path": file_path.to_str().unwrap(),
        "old_text": "original",
        "new_text": "modified"
    });
    
    let result = execute_local_tool("edit_file", args);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Safety validation failed"));
    
    Ok(())
}

#[tokio::test] 
async fn test_file_size_limits() -> Result<()> {
    let temp_dir = tempdir()?;
    let large_file = temp_dir.path().join("large.txt");
    
    // Create a file larger than the read limit
    let large_content = "x".repeat(60 * 1024 * 1024); // 60MB
    fs::write(&large_file, large_content)?;
    
    let args = json!({
        "path": large_file.to_str().unwrap()
    });
    
    let result = execute_local_tool("read_file", args);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("file too large"));
    
    Ok(())
}

#[test]
fn test_tool_list_consistency() {
    // Ensure all tools in the list are actually implemented
    let tools = list_local_tools();
    
    for tool_name in &tools {
        let result = execute_local_tool(tool_name, json!({}));
        // Should fail with argument errors, not unknown tool errors
        if result.is_err() {
            let error_msg = result.unwrap_err().to_string();
            assert!(!error_msg.contains("Unknown local tool"), 
                   "Tool '{}' is listed but not implemented", tool_name);
        }
    }
}