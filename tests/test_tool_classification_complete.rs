use prompt_manager::tool_safety::{ToolSafetyValidator, ToolSafetyLevel};
use prompt_manager::local_tools::list_local_tools;

/// Test that all available local tools are properly classified
#[test]
fn test_all_local_tools_classified() {
    let validator = ToolSafetyValidator::new(false, false);
    let all_tools = list_local_tools();
    
    println!("Testing classification for {} local tools:", all_tools.len());
    
    for tool in &all_tools {
        let classification = validator.classify_tool(tool);
        println!("  {} -> {:?}", tool, classification);
        
        // Verify each tool has an appropriate classification
        match tool.as_str() {
            // Safe tools (read-only operations)
            "list_directory" | "read_file" | "search_files" | "glob_files" 
            | "web_fetch" | "create_tasks" | "update_tasks" | "read_many_files"
            | "diff_files" | "batch_file_search" => {
                assert_eq!(classification, ToolSafetyLevel::Safe, 
                    "Tool '{}' should be classified as Safe", tool);
            },
            
            // Requires approval (file modification)
            "edit_file" | "write_file" | "multiedit_file" | "download_file" => {
                assert_eq!(classification, ToolSafetyLevel::RequiresApproval,
                    "Tool '{}' should be classified as RequiresApproval", tool);
            },
            
            // Dangerous (system operations)
            "execute_command" | "delete_path" => {
                assert_eq!(classification, ToolSafetyLevel::Dangerous,
                    "Tool '{}' should be classified as Dangerous", tool);
            },
            
            // Unknown tools should default to RequiresApproval
            _ => {
                assert_eq!(classification, ToolSafetyLevel::RequiresApproval,
                    "Unknown tool '{}' should default to RequiresApproval", tool);
            }
        }
    }
    
    println!("✅ All {} tools have been properly classified", all_tools.len());
}

/// Test specific tool classifications
#[test]
fn test_specific_tool_classifications() {
    let validator = ToolSafetyValidator::new(false, false);
    
    // Test some specific cases
    assert_eq!(validator.classify_tool("list_directory"), ToolSafetyLevel::Safe);
    assert_eq!(validator.classify_tool("read_file"), ToolSafetyLevel::Safe);
    assert_eq!(validator.classify_tool("web_fetch"), ToolSafetyLevel::Safe);
    assert_eq!(validator.classify_tool("read_many_files"), ToolSafetyLevel::Safe);
    assert_eq!(validator.classify_tool("diff_files"), ToolSafetyLevel::Safe);
    assert_eq!(validator.classify_tool("batch_file_search"), ToolSafetyLevel::Safe);
    
    assert_eq!(validator.classify_tool("write_file"), ToolSafetyLevel::RequiresApproval);
    assert_eq!(validator.classify_tool("edit_file"), ToolSafetyLevel::RequiresApproval);
    assert_eq!(validator.classify_tool("multiedit_file"), ToolSafetyLevel::RequiresApproval);
    assert_eq!(validator.classify_tool("download_file"), ToolSafetyLevel::RequiresApproval);
    
    assert_eq!(validator.classify_tool("execute_command"), ToolSafetyLevel::Dangerous);
    assert_eq!(validator.classify_tool("delete_path"), ToolSafetyLevel::Dangerous);
    
    // Test unknown tool
    assert_eq!(validator.classify_tool("unknown_tool"), ToolSafetyLevel::RequiresApproval);
}

/// Test MCP tools that might be available
#[test]
fn test_mcp_tool_classifications() {
    let validator = ToolSafetyValidator::new(false, false);
    
    // MCP tools that were seen in the logs
    assert_eq!(validator.classify_tool("list_allowed_directories"), ToolSafetyLevel::Safe);
    
    // Test some hypothetical MCP tools to ensure they get proper defaults
    assert_eq!(validator.classify_tool("hypothetical_read_tool"), ToolSafetyLevel::RequiresApproval);
    assert_eq!(validator.classify_tool("hypothetical_write_tool"), ToolSafetyLevel::RequiresApproval);
}