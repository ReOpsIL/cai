use anyhow::Result;
use tempfile::{tempdir, TempDir};
use std::fs;
use prompt_manager::{PromptManager, Prompt};

/// Setup test directory for prompt management tests
fn setup_prompt_management_test() -> Result<(TempDir, PromptManager)> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    let test_yaml = r#"name: "Management Test"
description: "Test prompt management operations"
subjects:
  - name: "Existing Subject"
    prompts:
      - title: "Existing prompt"
        content: "This is an existing prompt for testing updates"
        score: 2
        id: "existing-prompt-123"
  - name: "Another Subject"
    prompts:
      - title: "Another prompt"
        content: "Another existing prompt"
        score: 1
        id: "another-prompt-456"
"#;

    let test_file_path = prompts_dir.join("management_test.yaml");
    fs::write(&test_file_path, test_yaml)?;

    let manager = PromptManager::load_from_directory(&prompts_dir)?;
    Ok((temp_dir, manager))
}

#[test]
fn test_add_prompt_to_existing_subject() -> Result<()> {
    let (_temp_dir, mut manager) = setup_prompt_management_test()?;

    let new_prompt = Prompt {
        title: "New test prompt".to_string(),
        content: "This is a newly added prompt".to_string(),
        score: 0,
        id: "new-prompt-789".to_string(),
    };

    // Add prompt to existing subject
    manager.add_prompt_to_subject("management_test", "Existing Subject", new_prompt.clone())?;

    // Verify the prompt was added
    let prompt_data = manager.get_by_file_name("management_test").unwrap();
    let existing_subject = prompt_data.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Existing Subject")
        .unwrap();

    assert_eq!(existing_subject.prompts.len(), 2);
    
    let added_prompt = existing_subject.prompts
        .iter()
        .find(|p| p.id == "new-prompt-789")
        .unwrap();
    
    assert_eq!(added_prompt.title, "New test prompt");
    assert_eq!(added_prompt.content, "This is a newly added prompt");
    assert_eq!(added_prompt.score, 0);

    Ok(())
}

#[test]
fn test_add_prompt_to_new_subject() -> Result<()> {
    let (_temp_dir, mut manager) = setup_prompt_management_test()?;

    let new_prompt = Prompt {
        title: "Prompt for new subject".to_string(),
        content: "This prompt creates a new subject".to_string(),
        score: 0,
        id: "new-subject-prompt-999".to_string(),
    };

    // Add prompt to non-existing subject (should create the subject)
    manager.add_prompt_to_subject("management_test", "Brand New Subject", new_prompt.clone())?;

    // Verify the subject was created with the prompt
    let prompt_data = manager.get_by_file_name("management_test").unwrap();
    assert_eq!(prompt_data.prompt_file.subjects.len(), 3);

    let new_subject = prompt_data.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Brand New Subject")
        .unwrap();

    assert_eq!(new_subject.prompts.len(), 1);
    assert_eq!(new_subject.prompts[0].id, "new-subject-prompt-999");

    Ok(())
}

#[test]
fn test_update_prompt_content() -> Result<()> {
    let (_temp_dir, mut manager) = setup_prompt_management_test()?;

    let updated_content = "This is the updated content for the existing prompt";

    // Update existing prompt
    manager.update_prompt(
        "management_test",
        "Existing Subject",
        "existing-prompt-123",
        updated_content.to_string()
    )?;

    // Verify the prompt was updated
    let prompt_data = manager.get_by_file_name("management_test").unwrap();
    let existing_subject = prompt_data.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Existing Subject")
        .unwrap();
    
    let updated_prompt = existing_subject.prompts
        .iter()
        .find(|p| p.id == "existing-prompt-123")
        .unwrap();

    assert_eq!(updated_prompt.content, updated_content);
    assert_eq!(updated_prompt.score, 2); // Score should remain unchanged
    assert_eq!(updated_prompt.title, "Existing prompt"); // Title should remain unchanged

    Ok(())
}

#[test]
fn test_update_prompt_errors() -> Result<()> {
    let (_temp_dir, mut manager) = setup_prompt_management_test()?;

    // Test updating non-existent file
    let result = manager.update_prompt(
        "nonexistent_file",
        "Existing Subject",
        "existing-prompt-123",
        "new content".to_string()
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Prompt not found"));

    // Test updating non-existent subject
    let result = manager.update_prompt(
        "management_test",
        "Nonexistent Subject",
        "existing-prompt-123",
        "new content".to_string()
    );
    assert!(result.is_err());

    // Test updating non-existent prompt ID
    let result = manager.update_prompt(
        "management_test",
        "Existing Subject",
        "nonexistent-id",
        "new content".to_string()
    );
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_add_prompt_errors() -> Result<()> {
    let (_temp_dir, mut manager) = setup_prompt_management_test()?;

    let test_prompt = Prompt {
        title: "Test".to_string(),
        content: "Test content".to_string(),
        score: 0,
        id: "test-id".to_string(),
    };

    // Test adding to non-existent file
    let result = manager.add_prompt_to_subject("nonexistent_file", "Subject", test_prompt);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("File 'nonexistent_file' not found"));

    Ok(())
}

#[test]
fn test_ai_generated_file_creation() -> Result<()> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Start with empty directory
    let mut manager = PromptManager::load_from_directory(&prompts_dir)?;
    assert_eq!(manager.prompts.len(), 0);

    // Get or create AI generated file
    let ai_file_name = manager.get_or_create_ai_generated_file()?;
    assert_eq!(ai_file_name, "ai_generated");

    // Verify the file was created and added to manager
    assert_eq!(manager.prompts.len(), 1);
    let ai_file = manager.get_by_file_name("ai_generated").unwrap();
    assert_eq!(ai_file.prompt_file.name, "AI Generated");
    assert_eq!(ai_file.prompt_file.subjects.len(), 0);

    // The AI generated file is added to the manager in memory
    // In production, it would be written to prompts/ai_generated.yaml

    Ok(())
}

#[test]
fn test_ai_generated_file_already_exists() -> Result<()> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Create AI generated file manually first
    let ai_yaml = r#"name: "AI Generated"
description: "Prompts automatically generated through chat interactions"
subjects:
  - name: "Existing AI Subject"
    prompts:
      - title: "Existing AI prompt"
        content: "This was already here"
        score: 5
        id: "existing-ai-123"
"#;

    let ai_file_path = prompts_dir.join("ai_generated.yaml");
    fs::write(&ai_file_path, ai_yaml)?;

    let mut manager = PromptManager::load_from_directory(&prompts_dir)?;
    assert_eq!(manager.prompts.len(), 1);

    // Get or create should return existing file
    let ai_file_name = manager.get_or_create_ai_generated_file()?;
    assert_eq!(ai_file_name, "ai_generated");

    // Should still have only one file
    assert_eq!(manager.prompts.len(), 1);

    // Verify existing content is preserved
    let ai_file = manager.get_by_file_name("ai_generated").unwrap();
    assert_eq!(ai_file.prompt_file.subjects.len(), 1);
    assert_eq!(ai_file.prompt_file.subjects[0].prompts[0].score, 5);

    Ok(())
}

#[test]
fn test_add_prompt_persistence() -> Result<()> {
    let (temp_dir, mut manager) = setup_prompt_management_test()?;

    let new_prompt = Prompt {
        title: "Persistent prompt".to_string(),
        content: "This prompt should persist".to_string(),
        score: 0,
        id: "persistent-prompt-888".to_string(),
    };

    // Add prompt
    manager.add_prompt_to_subject("management_test", "Existing Subject", new_prompt.clone())?;

    // Reload manager from disk
    let reloaded_manager = PromptManager::load_from_directory(temp_dir.path().join("prompts"))?;

    // Verify persistence
    let prompt_data = reloaded_manager.get_by_file_name("management_test").unwrap();
    let existing_subject = prompt_data.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Existing Subject")
        .unwrap();

    let persistent_prompt = existing_subject.prompts
        .iter()
        .find(|p| p.id == "persistent-prompt-888")
        .unwrap();

    assert_eq!(persistent_prompt.title, "Persistent prompt");
    assert_eq!(persistent_prompt.content, "This prompt should persist");

    Ok(())
}

#[test]
fn test_update_prompt_persistence() -> Result<()> {
    let (temp_dir, mut manager) = setup_prompt_management_test()?;

    let updated_content = "This content was updated and should persist";

    // Update prompt
    manager.update_prompt(
        "management_test",
        "Existing Subject",
        "existing-prompt-123",
        updated_content.to_string()
    )?;

    // Reload manager from disk
    let reloaded_manager = PromptManager::load_from_directory(temp_dir.path().join("prompts"))?;

    // Verify persistence
    let prompt_data = reloaded_manager.get_by_file_name("management_test").unwrap();
    let existing_subject = prompt_data.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Existing Subject")
        .unwrap();

    let updated_prompt = existing_subject.prompts
        .iter()
        .find(|p| p.id == "existing-prompt-123")
        .unwrap();

    assert_eq!(updated_prompt.content, updated_content);

    Ok(())
}