use anyhow::Result;
use tempfile::{tempdir, TempDir};
use std::fs;
use prompt_manager::{PromptManager, calculate_text_similarity};

/// Setup test directory for edge case testing
fn setup_edge_case_test() -> Result<(TempDir, PromptManager)> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    let test_yaml = r#"name: "Edge Case Test"
description: "Test edge cases and error conditions"
subjects:
  - name: "Empty Subject"
    prompts: []
  - name: "Special Characters"
    prompts:
      - title: "Prompt with special chars: !@#$%^&*()"
        content: "Content with emojis 🚀🔥💯 and unicode characters: àáâãäåæçèéêë"
        score: 1
        id: "special-chars-001"
      - title: "Very long title that exceeds normal length limits and contains many words that should be handled gracefully"
        content: "Very long content that spans multiple lines and contains various formatting elements including newlines\nand tabs\tand other whitespace characters that might cause parsing issues."
        score: 999999
        id: "long-content-002"
  - name: "Edge Cases"
    prompts:
      - title: ""
        content: ""
        score: 0
        id: "empty-fields-003"
      - title: "Single char"
        content: "X"
        score: 0
        id: "minimal-content-004"
"#;

    let test_file_path = prompts_dir.join("edge_case_test.yaml");
    fs::write(&test_file_path, test_yaml)?;

    let manager = PromptManager::load_from_directory(&prompts_dir)?;
    Ok((temp_dir, manager))
}

#[test]
fn test_empty_text_similarity() {
    // Test empty strings
    assert_eq!(calculate_text_similarity("", ""), 1.0);
    assert_eq!(calculate_text_similarity("hello", ""), 0.0);
    assert_eq!(calculate_text_similarity("", "world"), 0.0);
}

#[test]
fn test_single_character_similarity() {
    // Test single characters
    assert_eq!(calculate_text_similarity("a", "a"), 1.0);
    assert!(calculate_text_similarity("a", "b") < 1.0);
    assert!(calculate_text_similarity("a", "ab") > 0.0);
}

#[test]
fn test_unicode_and_special_characters() -> Result<()> {
    let (_temp_dir, manager) = setup_edge_case_test()?;

    // Test searching with unicode characters
    let similar_prompts = manager.find_similar_prompts("émojis and unicode àáâã", 0.3);
    assert!(!similar_prompts.is_empty());

    let unicode_match = similar_prompts.iter()
        .find(|p| p.prompt.id == "special-chars-001");
    assert!(unicode_match.is_some());

    Ok(())
}

#[test]
fn test_very_long_content() -> Result<()> {
    let (_temp_dir, manager) = setup_edge_case_test()?;

    // Test finding similar prompts with very long content
    let long_task = "This is a very long task description that spans multiple lines and contains various elements that might cause issues with similarity detection and processing when dealing with extremely long content that exceeds normal expectations";
    
    let similar_prompts = manager.find_similar_prompts(long_task, 0.1);
    
    // Should handle long content gracefully
    for prompt in similar_prompts {
        assert!(prompt.similarity_score >= 0.0);
        assert!(prompt.similarity_score <= 1.0);
    }

    Ok(())
}

#[test]
fn test_empty_fields_handling() -> Result<()> {
    let (_temp_dir, manager) = setup_edge_case_test()?;

    // Test prompts with empty title and content
    let edge_case_file = manager.get_by_file_name("edge_case_test").unwrap();
    let edge_cases_subject = edge_case_file.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Edge Cases")
        .unwrap();

    let empty_prompt = edge_cases_subject.prompts
        .iter()
        .find(|p| p.id == "empty-fields-003")
        .unwrap();

    assert_eq!(empty_prompt.title, "");
    assert_eq!(empty_prompt.content, "");
    assert_eq!(empty_prompt.score, 0);

    Ok(())
}

#[test]
fn test_negative_and_extreme_scores() -> Result<()> {
    let (_temp_dir, mut manager) = setup_edge_case_test()?;

    // Test incrementing a zero score
    manager.increment_prompt_score("edge_case_test", "Edge Cases", "minimal-content-004")?;

    let edge_case_file = manager.get_by_file_name("edge_case_test").unwrap();
    let edge_cases_subject = edge_case_file.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Edge Cases")
        .unwrap();

    let zero_prompt = edge_cases_subject.prompts
        .iter()
        .find(|p| p.id == "minimal-content-004")
        .unwrap();

    assert_eq!(zero_prompt.score, 1); // 0 + 1 = 1

    // Test incrementing a very large score
    manager.increment_prompt_score("edge_case_test", "Special Characters", "long-content-002")?;

    // Re-fetch the data after the increment
    let edge_case_file_2 = manager.get_by_file_name("edge_case_test").unwrap();
    let special_subject = edge_case_file_2.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Special Characters")
        .unwrap();

    let large_score_prompt = special_subject.prompts
        .iter()
        .find(|p| p.id == "long-content-002")
        .unwrap();

    assert_eq!(large_score_prompt.score, 1000000); // 999999 + 1

    Ok(())
}

#[test]
fn test_empty_subject_handling() -> Result<()> {
    let (_temp_dir, manager) = setup_edge_case_test()?;

    let edge_case_file = manager.get_by_file_name("edge_case_test").unwrap();
    let empty_subject = edge_case_file.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Empty Subject")
        .unwrap();

    assert_eq!(empty_subject.prompts.len(), 0);

    // Test finding similar prompts when empty subjects exist
    let similar_prompts = manager.find_similar_prompts("any content", 0.0);
    
    // Should not crash or cause issues
    for prompt in similar_prompts {
        assert!(!prompt.subject_name.is_empty());
    }

    Ok(())
}

#[test]
fn test_invalid_file_operations() -> Result<()> {
    let (_temp_dir, mut manager) = setup_edge_case_test()?;

    // Test operations with empty strings
    let result = manager.increment_prompt_score("", "", "");
    assert!(result.is_err());

    let result = manager.update_prompt("", "", "", "".to_string());
    assert!(result.is_err());

    // Test operations with whitespace-only strings
    let result = manager.increment_prompt_score("   ", "   ", "   ");
    assert!(result.is_err());

    // Test operations with very long strings
    let very_long_string = "x".repeat(10000);
    let result = manager.increment_prompt_score(&very_long_string, &very_long_string, &very_long_string);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_similarity_edge_cases() {
    // Test identical very long strings
    let long_string = "a".repeat(1000);
    assert_eq!(calculate_text_similarity(&long_string, &long_string), 1.0);

    // Test completely different strings of same length
    let string1 = "a".repeat(100);
    let string2 = "b".repeat(100);
    assert_eq!(calculate_text_similarity(&string1, &string2), 0.0);

    // Test strings with repeated patterns
    let pattern1 = "abcd".repeat(50);
    let pattern2 = "abce".repeat(50);
    let similarity = calculate_text_similarity(&pattern1, &pattern2);
    assert!(similarity > 0.7); // Should be quite similar
    assert!(similarity < 1.0);

    // Test strings with whitespace differences
    let text1 = "hello world test";
    let text2 = "hello\tworld\ntest";
    let similarity = calculate_text_similarity(text1, text2);
    assert!(similarity > 0.8); // Should be very similar despite whitespace
}

#[test]
fn test_malformed_prompt_ids() -> Result<()> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Create YAML with unusual ID formats
    let unusual_ids_yaml = r#"name: "Unusual IDs"
description: "Test unusual ID formats"
subjects:
  - name: "ID Tests"
    prompts:
      - title: "Numeric ID"
        content: "Test numeric ID"
        score: 0
        id: "123456789"
      - title: "Special char ID"
        content: "Test special character ID"
        score: 0
        id: "id-with-special@chars#$%"
      - title: "Unicode ID"
        content: "Test unicode ID"
        score: 0
        id: "测试-ид-テスト"
      - title: "Empty ID"
        content: "Test empty ID"
        score: 0
        id: ""
"#;

    let test_file_path = prompts_dir.join("unusual_ids.yaml");
    fs::write(&test_file_path, unusual_ids_yaml)?;

    let mut manager = PromptManager::load_from_directory(&prompts_dir)?;

    // Test operations with unusual IDs
    let result = manager.increment_prompt_score("unusual_ids", "ID Tests", "123456789");
    assert!(result.is_ok());

    let result = manager.increment_prompt_score("unusual_ids", "ID Tests", "id-with-special@chars#$%");
    assert!(result.is_ok());

    let result = manager.increment_prompt_score("unusual_ids", "ID Tests", "测试-ид-テスト");
    assert!(result.is_ok());

    // Empty ID should still work
    let result = manager.increment_prompt_score("unusual_ids", "ID Tests", "");
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_concurrent_file_operations() -> Result<()> {
    let (_temp_dir, mut manager) = setup_edge_case_test()?;

    // Simulate rapid consecutive operations
    for i in 0..10 {
        let result = manager.increment_prompt_score("edge_case_test", "Special Characters", "special-chars-001");
        assert!(result.is_ok(), "Failed on iteration {}", i);
    }

    // Verify all operations succeeded
    let edge_case_file = manager.get_by_file_name("edge_case_test").unwrap();
    let special_subject = edge_case_file.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Special Characters")
        .unwrap();

    let special_prompt = special_subject.prompts
        .iter()
        .find(|p| p.id == "special-chars-001")
        .unwrap();

    assert_eq!(special_prompt.score, 11); // 1 + 10 increments

    Ok(())
}

#[test]
fn test_memory_usage_with_large_datasets() -> Result<()> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Create a YAML file with many prompts
    let mut large_yaml = String::from(r#"name: "Large Dataset"
description: "Test with many prompts"
subjects:
  - name: "Large Subject"
    prompts:
"#);

    for i in 0..1000 {
        large_yaml.push_str(&format!(r#"      - title: "Prompt {}"
        content: "This is test prompt number {} with some content to make it realistic"
        score: {}
        id: "large-prompt-{:04}"
"#, i, i, i % 10, i));
    }

    fs::write(prompts_dir.join("large_dataset.yaml"), large_yaml)?;

    // Load and test operations
    let mut manager = PromptManager::load_from_directory(&prompts_dir)?;
    
    // Test similarity search with large dataset
    let similar_prompts = manager.find_similar_prompts("test prompt with content", 0.3);
    assert!(!similar_prompts.is_empty());

    // Test scoring operation
    let result = manager.increment_prompt_score("large_dataset", "Large Subject", "large-prompt-0500");
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_file_system_edge_cases() -> Result<()> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Create file with unusual name
    let unusual_filename = "file with spaces & special chars!.yaml";
    let test_yaml = r#"name: "Unusual Filename"
description: "Test unusual filename handling"
subjects:
  - name: "Test"
    prompts:
      - title: "Test prompt"
        content: "Test content"
        score: 0
        id: "test-001"
"#;

    fs::write(prompts_dir.join(unusual_filename), test_yaml)?;

    // Should load successfully
    let manager = PromptManager::load_from_directory(&prompts_dir)?;
    assert_eq!(manager.prompts.len(), 1);

    Ok(())
}

#[test]
fn test_url_edge_cases() -> Result<()> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Test various URL edge cases
    let url_edge_cases_yaml = r#"name: "URL Edge Cases"
description: "Test URL handling edge cases"
subjects:
  - name: "URL Tests"
    prompts:
      - title: "Invalid URL"
        content: "not-a-valid-url"
        score: 0
        id: "invalid-url-001"
      - title: "File URL nonexistent"
        content: "file://nonexistent_file.txt"
        score: 0
        id: "nonexistent-file-002"
      - title: "HTTP URL nonexistent"
        content: "https://nonexistent-domain-12345.com/prompt.md"
        score: 0
        id: "nonexistent-http-003"
"#;

    fs::write(prompts_dir.join("url_edge_cases.yaml"), url_edge_cases_yaml)?;
    let manager = PromptManager::load_from_directory(&prompts_dir)?;

    // Test similarity search should handle URL resolution failures gracefully
    let similar_prompts = manager.find_similar_prompts("test content", 0.1);
    
    // Should not crash, even if URL resolution fails
    for prompt in similar_prompts {
        assert!(prompt.similarity_score >= 0.0);
        assert!(prompt.similarity_score <= 1.0);
    }

    Ok(())
}