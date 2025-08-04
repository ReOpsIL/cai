use anyhow::Result;
use tempfile::{tempdir, TempDir};
use std::fs;
use prompt_manager::{PromptManager, Prompt, Subject, PromptFile};

/// Helper function to create a temporary test directory with sample YAML files
fn setup_test_directory() -> Result<(TempDir, PromptManager)> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Create a test YAML file with prompts that have scores
    let test_yaml = r#"name: "Test Prompts"
description: "Test prompts for scoring validation"
subjects:
  - name: "Testing"
    prompts:
      - title: "Test basic functionality"
        content: "This is a test prompt for basic functionality testing"
        score: 5
        id: "test-basic-123"
      - title: "Test advanced features"
        content: "This is a test prompt for advanced feature testing"
        score: 0
        id: "test-advanced-456"
  - name: "Debugging"
    prompts:
      - title: "Debug application errors"
        content: "Analyze application logs and identify error patterns"
        score: 3
        id: "debug-errors-789"
"#;

    let test_file_path = prompts_dir.join("test_prompts.yaml");
    fs::write(&test_file_path, test_yaml)?;

    let manager = PromptManager::load_from_directory(&prompts_dir)?;
    Ok((temp_dir, manager))
}

#[test]
fn test_prompt_score_increment() -> Result<()> {
    let (_temp_dir, mut manager) = setup_test_directory()?;

    // Test incrementing score of an existing prompt
    manager.increment_prompt_score("test_prompts", "Testing", "test-basic-123")?;

    // Verify the score was incremented
    let prompt_data = manager.get_by_file_name("test_prompts").unwrap();
    let testing_subject = prompt_data.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Testing")
        .unwrap();
    let basic_prompt = testing_subject.prompts
        .iter()
        .find(|p| p.id == "test-basic-123")
        .unwrap();

    assert_eq!(basic_prompt.score, 6); // Should be incremented from 5 to 6

    // Test incrementing a prompt with score 0
    manager.increment_prompt_score("test_prompts", "Testing", "test-advanced-456")?;

    // Re-fetch the data after the second increment
    let prompt_data_2 = manager.get_by_file_name("test_prompts").unwrap();
    let testing_subject_2 = prompt_data_2.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Testing")
        .unwrap();
    let advanced_prompt = testing_subject_2.prompts
        .iter()
        .find(|p| p.id == "test-advanced-456")
        .unwrap();

    assert_eq!(advanced_prompt.score, 1); // Should be incremented from 0 to 1

    Ok(())
}

#[test]
fn test_prompt_score_increment_error_handling() -> Result<()> {
    let (_temp_dir, mut manager) = setup_test_directory()?;

    // Test error when file doesn't exist
    let result = manager.increment_prompt_score("nonexistent", "Testing", "test-basic-123");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Prompt not found"));

    // Test error when subject doesn't exist
    let result = manager.increment_prompt_score("test_prompts", "NonexistentSubject", "test-basic-123");
    assert!(result.is_err());

    // Test error when prompt ID doesn't exist
    let result = manager.increment_prompt_score("test_prompts", "Testing", "nonexistent-id");
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_multiple_score_increments() -> Result<()> {
    let (_temp_dir, mut manager) = setup_test_directory()?;

    // Increment the same prompt multiple times
    for _ in 0..5 {
        manager.increment_prompt_score("test_prompts", "Debugging", "debug-errors-789")?;
    }

    // Verify the score was incremented correctly
    let prompt_data = manager.get_by_file_name("test_prompts").unwrap();
    let debugging_subject = prompt_data.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Debugging")
        .unwrap();
    let debug_prompt = debugging_subject.prompts
        .iter()
        .find(|p| p.id == "debug-errors-789")
        .unwrap();

    assert_eq!(debug_prompt.score, 8); // Should be 3 + 5 = 8

    Ok(())
}

#[test]
fn test_score_persistence() -> Result<()> {
    let (temp_dir, mut manager) = setup_test_directory()?;

    // Increment a score
    manager.increment_prompt_score("test_prompts", "Testing", "test-basic-123")?;

    // Reload the manager from the same directory
    let reloaded_manager = PromptManager::load_from_directory(temp_dir.path().join("prompts"))?;

    // Verify the score persisted
    let prompt_data = reloaded_manager.get_by_file_name("test_prompts").unwrap();
    let testing_subject = prompt_data.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Testing")
        .unwrap();
    let basic_prompt = testing_subject.prompts
        .iter()
        .find(|p| p.id == "test-basic-123")
        .unwrap();

    assert_eq!(basic_prompt.score, 6); // Should be persisted as 6

    Ok(())
}

#[test]
fn test_yaml_serialization_with_scores() -> Result<()> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Create a prompt file manually
    let prompt_file = PromptFile {
        name: "Serialization Test".to_string(),
        description: "Test YAML serialization with scores".to_string(),
        subjects: vec![
            Subject {
                name: "Test Subject".to_string(),
                prompts: vec![
                    Prompt {
                        title: "Test Prompt".to_string(),
                        content: "Test content".to_string(),
                        score: 42,
                        id: "test-id-999".to_string(),
                    }
                ],
            }
        ],
    };

    let file_path = prompts_dir.join("serialization_test.yaml");
    let yaml_content = serde_yaml::to_string(&prompt_file)?;
    fs::write(&file_path, yaml_content)?;

    // Load it back and verify
    let manager = PromptManager::load_from_directory(&prompts_dir)?;
    let loaded_data = manager.get_by_file_name("serialization_test").unwrap();
    let loaded_prompt = &loaded_data.prompt_file.subjects[0].prompts[0];

    assert_eq!(loaded_prompt.score, 42);
    assert_eq!(loaded_prompt.id, "test-id-999");
    assert_eq!(loaded_prompt.title, "Test Prompt");

    Ok(())
}

#[test]
fn test_backward_compatibility_no_scores() -> Result<()> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Create a YAML file without score and id fields (backward compatibility)
    let old_format_yaml = r#"name: "Old Format"
description: "Test backward compatibility"
subjects:
  - name: "Legacy"
    prompts:
      - title: "Old prompt"
        content: "This prompt has no score or id fields"
"#;

    let test_file_path = prompts_dir.join("old_format.yaml");
    fs::write(&test_file_path, old_format_yaml)?;

    // Should load successfully with default values
    let manager = PromptManager::load_from_directory(&prompts_dir)?;
    let prompt_data = manager.get_by_file_name("old_format").unwrap();
    let prompt = &prompt_data.prompt_file.subjects[0].prompts[0];

    assert_eq!(prompt.score, 0); // Default score
    assert!(!prompt.id.is_empty()); // Should have generated UUID
    assert_eq!(prompt.title, "Old prompt");

    Ok(())
}