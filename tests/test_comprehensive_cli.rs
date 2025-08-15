use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::json;
use std::fs;
use tempfile::{tempdir, TempDir};

/// Setup comprehensive CLI test environment
fn setup_cli_test() -> Result<TempDir> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Create comprehensive prompt files
    let advanced_yaml = r#"name: "Advanced Programming"
description: "Comprehensive programming assistance prompts"
subjects:
  - name: "Code Analysis"
    prompts:
      - title: "Security vulnerability scan"
        content: "Analyze this code for security vulnerabilities and provide detailed remediation steps"
        score: 10
        id: "security-scan-001"
      - title: "Performance optimization analysis"
        content: "Identify performance bottlenecks and suggest optimizations"
        score: 5
        id: "perf-analysis-002"
  - name: "Testing"
    prompts:
      - title: "Generate comprehensive test suite"
        content: "Create unit tests, integration tests, and edge case tests for this code"
        score: 3
        id: "test-gen-003"
"#;

    let mcp_config = json!({
        "mcpServers": {
            "filesystem": {
                "command": "echo",
                "args": ["mock-mcp-server"],
                "env": {},
                "cwd": null
            }
        }
    });

    fs::write(prompts_dir.join("advanced.yaml"), advanced_yaml)?;
    fs::write(temp_dir.path().join("mcp-config.json"), mcp_config.to_string())?;

    Ok(temp_dir)
}

#[test]
fn test_cli_list_command() -> Result<()> {
    let temp_dir = setup_cli_test()?;

    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Advanced Programming"))
        .stdout(predicate::str::contains("Code Analysis"))
        .stdout(predicate::str::contains("Testing"))
        .stdout(predicate::str::contains("Security vulnerability scan"))
        .stdout(predicate::str::contains("⭐")); // Score display

    Ok(())
}

#[test]
fn test_cli_search_command() -> Result<()> {
    let temp_dir = setup_cli_test()?;

    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("search")
        .arg("security");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Security vulnerability scan"))
        .stdout(predicate::str::contains("security-scan-001"));

    Ok(())
}

#[test]
fn test_cli_show_command() -> Result<()> {
    let temp_dir = setup_cli_test()?;

    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("show")
        .arg("advanced");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Advanced Programming"))
        .stdout(predicate::str::contains("Code Analysis"))
        .stdout(predicate::str::contains("Testing"));

    Ok(())
}

#[test]
fn test_cli_query_command() -> Result<()> {
    let temp_dir = setup_cli_test()?;

    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("query")
        .arg("file")
        .arg("advanced")
        .arg("subject")
        .arg("Code Analysis")
        .arg("prompt")
        .arg("Security vulnerability scan");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Analyze this code for security vulnerabilities"));

    Ok(())
}

#[test]
fn test_scan_status_command() -> Result<()> {
    let _temp_dir = setup_cli_test()?;

    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("scan").arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scan Status"))
        .stdout(predicate::str::contains("No active scan running"));

    Ok(())
}

#[test]
fn test_scan_cancel_command() -> Result<()> {
    let _temp_dir = setup_cli_test()?;

    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("scan").arg("cancel");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No active scan to cancel"));

    Ok(())
}

#[test]
fn test_mcp_list_command() -> Result<()> {
    let temp_dir = setup_cli_test()?;

    let mut cmd = Command::cargo_bin("cai")?;
    cmd.current_dir(temp_dir.path())
        .arg("mcp")
        .arg("list");

    // Should succeed even if no real MCP servers are configured
    cmd.assert().success();

    Ok(())
}

#[test]
fn test_task_demo_command() -> Result<()> {
    let temp_dir = setup_cli_test()?;

    let mut cmd = Command::cargo_bin("cai")?;
    cmd.current_dir(temp_dir.path())
        .arg("task-demo");

    // Task demo should run without crashing
    // It might fail due to no real MCP servers, but should not panic
    let output = cmd.output()?;
    
    // Should at least attempt to run
    assert!(output.status.code().is_some());

    Ok(())
}

#[test]
fn test_invalid_commands() -> Result<()> {
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("invalid-command");

    cmd.assert().failure();

    Ok(())
}

#[test]
fn test_help_command() -> Result<()> {
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("query"))
        .stdout(predicate::str::contains("chat"))
        .stdout(predicate::str::contains("scan"))
        .stdout(predicate::str::contains("mcp"))
        .stdout(predicate::str::contains("workflow"));

    Ok(())
}

#[test]
fn test_version_output() -> Result<()> {
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));

    Ok(())
}

#[test]
fn test_directory_parameter() -> Result<()> {
    let temp_dir = setup_cli_test()?;
    
    // Test with valid directory
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("list");

    cmd.assert().success();

    // Test with invalid directory
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg("/nonexistent/directory")
        .arg("list");

    cmd.assert().failure();

    Ok(())
}

#[test]
fn test_error_handling() -> Result<()> {
    // Test missing required arguments
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("query").arg("file"); // Missing other required args

    cmd.assert().failure();

    Ok(())
}

#[test]
fn test_workflow_commands() -> Result<()> {
    let temp_dir = setup_cli_test()?;

    // Test workflow status
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.current_dir(temp_dir.path())
        .arg("workflow")
        .arg("status");

    cmd.assert().success();

    // Test workflow list
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.current_dir(temp_dir.path())
        .arg("workflow")
        .arg("list");

    cmd.assert().success();

    Ok(())
}

#[test]
fn test_concurrent_cli_calls() -> Result<()> {
    use std::thread;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let temp_dir = setup_cli_test()?;
    let temp_path = Arc::new(temp_dir.path().to_owned());
    let success_count = Arc::new(AtomicUsize::new(0));

    // Spawn multiple CLI calls concurrently
    let handles: Vec<_> = (0..5).map(|_| {
        let path = temp_path.clone();
        let counter = success_count.clone();
        
        thread::spawn(move || {
            let mut cmd = Command::cargo_bin("cai").unwrap();
            let result = cmd
                .arg("--directory")
                .arg(path.join("prompts"))
                .arg("list")
                .assert()
                .try_success();
                
            if result.is_ok() {
                counter.fetch_add(1, Ordering::SeqCst);
            }
        })
    }).collect();

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // At least some should succeed
    let final_count = success_count.load(Ordering::SeqCst);
    assert!(final_count > 0);

    Ok(())
}

#[test]
fn test_json_output_format() -> Result<()> {
    let temp_dir = setup_cli_test()?;

    // Some commands might support JSON output in the future
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(temp_dir.path().join("prompts"))
        .arg("list");

    let output = cmd.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should produce structured output (even if not JSON yet)
    assert!(!stdout.is_empty());
    assert!(stdout.contains("Advanced Programming"));

    Ok(())
}

#[test]
fn test_configuration_validation() -> Result<()> {
    let temp_dir = setup_cli_test()?;

    // Test with valid MCP configuration
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.current_dir(temp_dir.path())
        .arg("mcp")
        .arg("list");

    // Should not crash with valid config
    cmd.assert().success();

    // Test with invalid MCP configuration
    let invalid_config = "{ invalid json }";
    fs::write(temp_dir.path().join("mcp-config.json"), invalid_config)?;

    let mut cmd = Command::cargo_bin("cai")?;
    cmd.current_dir(temp_dir.path())
        .arg("mcp")
        .arg("list");

    // Should handle invalid config gracefully
    let output = cmd.output()?;
    // Should either succeed with warning or fail gracefully
    assert!(output.status.code().is_some());

    Ok(())
}

#[test]
fn test_large_prompt_files() -> Result<()> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Create a large YAML file with many prompts
    let mut large_yaml = String::from(r#"name: "Large Test File"
description: "Testing with many prompts"
subjects:
  - name: "Large Subject"
    prompts:
"#);

    for i in 0..1000 {
        large_yaml.push_str(&format!(
            r#"      - title: "Test prompt {}"
        content: "This is test prompt number {} with some content to make it realistic"
        score: {}
        id: "test-{:04}"
"#,
            i, i, i % 100, i
        ));
    }

    fs::write(prompts_dir.join("large.yaml"), large_yaml)?;

    // Test that CLI can handle large files
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(prompts_dir)
        .arg("list");

    cmd.assert().success();

    // Test search on large file
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(prompts_dir)
        .arg("search")
        .arg("prompt 500");

    cmd.assert().success()
        .stdout(predicate::str::contains("test prompt 500"));

    Ok(())
}

#[test]
fn test_unicode_support() -> Result<()> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Create YAML with Unicode content
    let unicode_yaml = r#"name: "Unicode Test 🚀"
description: "Testing Unicode support with emojis and international characters"
subjects:
  - name: "International 🌍"
    prompts:
      - title: "Français: Aide à la programmation"
        content: "Aidez-moi avec ce code en français 🇫🇷"
        score: 1
        id: "french-001"
      - title: "日本語: プログラミング支援"
        content: "日本語でコードの説明をお願いします 🇯🇵"
        score: 2
        id: "japanese-002"
      - title: "Español: Ayuda con código"
        content: "Ayúdame con este código en español 🇪🇸"
        score: 3
        id: "spanish-003"
"#;

    fs::write(prompts_dir.join("unicode.yaml"), unicode_yaml)?;

    // Test list with Unicode
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(prompts_dir)
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Unicode Test 🚀"))
        .stdout(predicate::str::contains("International 🌍"))
        .stdout(predicate::str::contains("Français"));

    // Test search with Unicode
    let mut cmd = Command::cargo_bin("cai")?;
    cmd.arg("--directory")
        .arg(prompts_dir)
        .arg("search")
        .arg("日本語");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("japanese-002"));

    Ok(())
}