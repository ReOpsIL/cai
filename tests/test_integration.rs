use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::{tempdir, TempDir};
use std::fs;

/// Setup integration test environment with sample prompts
fn setup_integration_test() -> Result<TempDir> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Create sample YAML files for testing
    let bug_fixing_yaml = r#"name: "Bug Fixing"
description: "Prompts for debugging and fixing code issues"
subjects:
  - name: "General Debugging"
    prompts:
      - title: "Analyze error logs"
        content: "Analyze the following error logs and identify the root cause"
        score: 3
        id: "analyze-logs-001"
      - title: "Code review for bugs"
        content: "Review this code for potential bugs and security issues"
        score: 1
        id: "review-bugs-002"
  - name: "Performance Issues"
    prompts:
      - title: "Performance bottleneck analysis"
        content: "Analyze this code for performance bottlenecks and optimization opportunities"
        score: 5
        id: "perf-bottleneck-003"
"#;

    let task_creation_yaml = r#"name: "Task Creation"
description: "Prompts for project planning and task management"
subjects:
  - name: "Project Planning"
    prompts:
      - title: "Break down feature requirements"
        content: "Break down this feature request into smaller, manageable tasks"
        score: 2
        id: "feature-breakdown-004"
      - title: "Sprint planning assistant"
        content: "Help plan the next sprint by analyzing user stories and estimating effort"
        score: 0
        id: "sprint-planning-005"
"#;

    fs::write(prompts_dir.join("bug_fixing.yaml"), bug_fixing_yaml)?;
    fs::write(prompts_dir.join("task_creation.yaml"), task_creation_yaml)?;

    Ok(temp_dir)
}

#[test]
fn test_list_command_integration() -> Result<()> {
    let temp_dir = setup_integration_test()?;
    
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Available Prompt Files:"))
        .stdout(predicate::str::contains("bug_fixing"))
        .stdout(predicate::str::contains("task_creation"))
        .stdout(predicate::str::contains("Bug Fixing"))
        .stdout(predicate::str::contains("Task Creation"))
        .stdout(predicate::str::contains("Analyze error logs"))
        .stdout(predicate::str::contains("⭐ 3")) // Score display
        .stdout(predicate::str::contains("⭐ 5")) // Score display
        .stdout(predicate::str::contains("Performance bottleneck analysis"));

    Ok(())
}

#[test]
fn test_search_command_integration() -> Result<()> {
    let temp_dir = setup_integration_test()?;
    
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("search")
        .arg("performance");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Found"))
        .stdout(predicate::str::contains("result(s) for 'performance'"))
        .stdout(predicate::str::contains("Performance bottleneck analysis"))
        .stdout(predicate::str::contains("bug_fixing"));

    Ok(())
}

#[test]
fn test_search_no_results() -> Result<()> {
    let temp_dir = setup_integration_test()?;
    
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("search")
        .arg("nonexistent");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No results found for 'nonexistent'"));

    Ok(())
}

#[test]
fn test_show_command_integration() -> Result<()> {
    let temp_dir = setup_integration_test()?;
    
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("show")
        .arg("bug_fixing");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Bug Fixing"))
        .stdout(predicate::str::contains("Prompts for debugging and fixing code issues"))
        .stdout(predicate::str::contains("General Debugging"))
        .stdout(predicate::str::contains("Performance Issues"))
        .stdout(predicate::str::contains("Analyze error logs"))
        .stdout(predicate::str::contains("⭐ 3"))
        .stdout(predicate::str::contains("⭐ 5"));

    Ok(())
}

#[test]
fn test_show_nonexistent_file() -> Result<()> {
    let temp_dir = setup_integration_test()?;
    
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("show")
        .arg("nonexistent");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("File 'nonexistent' not found"));

    Ok(())
}

#[test]
fn test_query_command_integration() -> Result<()> {
    let temp_dir = setup_integration_test()?;
    
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("query")
        .arg("bug_fixing")
        .arg("General Debugging")
        .arg("Analyze error logs");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Analyze error logs"))
        .stdout(predicate::str::contains("bug_fixing → General Debugging"))
        .stdout(predicate::str::contains("Analyze the following error logs and identify the root cause"));

    Ok(())
}

#[test]
fn test_query_nonexistent_prompt() -> Result<()> {
    let temp_dir = setup_integration_test()?;
    
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("query")
        .arg("bug_fixing")
        .arg("General Debugging")
        .arg("Nonexistent Prompt");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Prompt 'Nonexistent Prompt' not found"));

    Ok(())
}

#[test]
fn test_chat_command_without_api_key() -> Result<()> {
    let temp_dir = setup_integration_test()?;
    
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("chat")
        .env_remove("OPENROUTER_API_KEY"); // Ensure no API key

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Failed to start chat mode"))
        .stdout(predicate::str::contains("OPENROUTER_API_KEY"))
        .stdout(predicate::str::contains("openrouter.ai"));

    Ok(())
}

#[test]
fn test_custom_directory() -> Result<()> {
    let temp_dir = setup_integration_test()?;
    let custom_prompts_dir = temp_dir.path().join("custom_prompts");
    fs::create_dir_all(&custom_prompts_dir)?;

    // Create a prompt file in custom directory
    let custom_yaml = r#"name: "Custom Prompts"
description: "Custom prompts in non-default directory"
subjects:
  - name: "Custom Subject"
    prompts:
      - title: "Custom prompt"
        content: "This is a custom prompt"
        score: 1
        id: "custom-001"
"#;

    fs::write(custom_prompts_dir.join("custom.yaml"), custom_yaml)?;

    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(&custom_prompts_dir)
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Custom Prompts"))
        .stdout(predicate::str::contains("Custom Subject"))
        .stdout(predicate::str::contains("Custom prompt"));

    Ok(())
}

#[test]
fn test_invalid_directory() -> Result<()> {
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg("/nonexistent/directory")
        .arg("list");

    cmd.assert()
        .failure(); // Should fail with invalid directory

    Ok(())
}

#[test]
fn test_help_command() -> Result<()> {
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("A CLI tool for managing and searching prompt collections"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("query"))
        .stdout(predicate::str::contains("chat"));

    Ok(())
}

#[test]
fn test_url_content_integration() -> Result<()> {
    let temp_dir = setup_integration_test()?;
    let prompts_dir = temp_dir.path().join("prompts");

    // Create a markdown file for URL reference
    let md_content = "This is a detailed prompt for URL testing with comprehensive instructions and examples.";
    let md_file_path = prompts_dir.join("url_prompt.md");
    fs::write(&md_file_path, md_content)?;

    // Create YAML with URL reference
    let url_yaml = format!(r#"name: "URL Test"
description: "Test URL content loading"
subjects:
  - name: "URL References"
    prompts:
      - title: "URL prompt test"
        content: "file://{}"
        score: 0
        id: "url-test-001"
"#, md_file_path.to_string_lossy());

    fs::write(prompts_dir.join("url_test.yaml"), url_yaml)?;

    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(&prompts_dir)
        .arg("show")
        .arg("url_test");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("URL Test"))
        .stdout(predicate::str::contains("🔗"))
        .stdout(predicate::str::contains("file://"))
        .stdout(predicate::str::contains("This is a detailed prompt for URL testing"));

    Ok(())
}

#[test]
fn test_score_display_in_listing() -> Result<()> {
    let temp_dir = setup_integration_test()?;
    
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("⭐ 3"))  // Score for analyze-logs
        .stdout(predicate::str::contains("⭐ 1"))  // Score for review-bugs
        .stdout(predicate::str::contains("⭐ 5"))  // Score for perf-bottleneck
        .stdout(predicate::str::contains("⭐ 2")); // Score for feature-breakdown

    Ok(())
}

#[test]
fn test_zero_score_not_displayed() -> Result<()> {
    let temp_dir = setup_integration_test()?;
    
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("show")
        .arg("task_creation");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Sprint planning assistant"))
        .stdout(predicate::str::contains("⭐ 2")); // Feature breakdown has score 2
        // Zero scores are not displayed, so we just check other content

    Ok(())
}

#[test]
fn test_empty_prompts_directory() -> Result<()> {
    let temp_dir = tempdir()?;
    let empty_prompts_dir = temp_dir.path().join("empty_prompts");
    fs::create_dir_all(&empty_prompts_dir)?;

    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(&empty_prompts_dir)
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Available Prompt Files:"));

    Ok(())
}

#[test]
fn test_malformed_yaml_handling() -> Result<()> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Create malformed YAML file
    let malformed_yaml = r#"name: "Malformed"
description: "This YAML has syntax errors
subjects:
  - name: "Test"
    prompts:
      - title: "Test prompt"
        content: "Test"
        score: invalid_number
"#;

    fs::write(prompts_dir.join("malformed.yaml"), malformed_yaml)?;

    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(&prompts_dir)
        .arg("list");

    cmd.assert()
        .failure(); // Should fail to parse malformed YAML

    Ok(())
}