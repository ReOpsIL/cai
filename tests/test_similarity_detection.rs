use anyhow::Result;
use tempfile::{tempdir, TempDir};
use std::fs;
use prompt_manager::{PromptManager, calculate_text_similarity};

/// Setup test directory with diverse prompts for similarity testing
fn setup_similarity_test_directory() -> Result<(TempDir, PromptManager)> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    let test_yaml = r#"name: "Similarity Test Prompts"
description: "Prompts for testing similarity detection"
subjects:
  - name: "Code Analysis"
    prompts:
      - title: "Analyze code for bugs"
        content: "Review this code for potential bugs, errors, and issues"
        score: 2
        id: "analyze-bugs-001"
      - title: "Code review for issues"
        content: "Examine this code for bugs, problems, and potential errors"
        score: 1
        id: "review-issues-002"
      - title: "Performance optimization"
        content: "Optimize this code for better performance and efficiency"
        score: 3
        id: "optimize-perf-003"
  - name: "Testing"
    prompts:
      - title: "Create unit tests"
        content: "Write comprehensive unit tests for this function"
        score: 0
        id: "unit-tests-004"
      - title: "Integration testing"
        content: "Design integration tests for the application"
        score: 1
        id: "integration-tests-005"
"#;

    let test_file_path = prompts_dir.join("similarity_test.yaml");
    fs::write(&test_file_path, test_yaml)?;

    let manager = PromptManager::load_from_directory(&prompts_dir)?;
    Ok((temp_dir, manager))
}

#[test]
fn test_text_similarity_calculation() {
    // Test identical texts
    let similarity = calculate_text_similarity("hello world", "hello world");
    assert_eq!(similarity, 1.0);

    // Test completely different texts
    let similarity = calculate_text_similarity("hello", "xyz");
    assert!(similarity < 0.5);

    // Test similar texts
    let similarity = calculate_text_similarity(
        "analyze code for bugs",
        "review code for errors"
    );
    assert!(similarity > 0.5);
    assert!(similarity < 1.0);

    // Test empty strings
    let similarity = calculate_text_similarity("", "");
    assert_eq!(similarity, 1.0);

    // Test one empty string
    let similarity = calculate_text_similarity("hello", "");
    assert_eq!(similarity, 0.0);
}

#[test]
fn test_find_similar_prompts_high_threshold() -> Result<()> {
    let (_temp_dir, manager) = setup_similarity_test_directory()?;

    // Search for a prompt very similar to existing one
    let task = "Review this code for potential bugs, errors, and issues";
    let similar_prompts = manager.find_similar_prompts(task, 0.8);

    // Should find the highly similar prompt
    assert!(!similar_prompts.is_empty());
    
    let best_match = &similar_prompts[0];
    assert!(best_match.similarity_score >= 0.8);
    assert_eq!(best_match.prompt.id, "analyze-bugs-001");

    Ok(())
}

#[test]
fn test_find_similar_prompts_medium_threshold() -> Result<()> {
    let (_temp_dir, manager) = setup_similarity_test_directory()?;

    // Search for a moderately similar prompt
    let task = "Examine code for problems and defects";
    let similar_prompts = manager.find_similar_prompts(task, 0.5);

    // Should find multiple similar prompts
    assert!(similar_prompts.len() >= 2);
    
    // Results should be sorted by similarity (highest first)
    for i in 0..similar_prompts.len()-1 {
        assert!(similar_prompts[i].similarity_score >= similar_prompts[i+1].similarity_score);
    }

    Ok(())
}

#[test]
fn test_find_similar_prompts_no_matches() -> Result<()> {
    let (_temp_dir, manager) = setup_similarity_test_directory()?;

    // Search for a completely unrelated task
    let task = "Calculate mathematical equations for quantum physics";
    let similar_prompts = manager.find_similar_prompts(task, 0.5);

    // Should find no similar prompts
    assert!(similar_prompts.is_empty());

    Ok(())
}

#[test]
fn test_find_similar_prompts_low_threshold() -> Result<()> {
    let (_temp_dir, manager) = setup_similarity_test_directory()?;

    // Search with very low threshold
    let task = "code analysis and review";
    let similar_prompts = manager.find_similar_prompts(task, 0.1);

    // Should find several prompts with low threshold
    assert!(similar_prompts.len() >= 3);
    
    // Verify all results meet the threshold
    for similar_prompt in similar_prompts {
        assert!(similar_prompt.similarity_score >= 0.1);
    }

    Ok(())
}

#[test]
fn test_similarity_with_different_subjects() -> Result<()> {
    let (_temp_dir, manager) = setup_similarity_test_directory()?;

    // Search for testing-related task
    let task = "Write unit tests for functions";
    let similar_prompts = manager.find_similar_prompts(task, 0.4);

    // Should find the testing-related prompt
    assert!(!similar_prompts.is_empty());
    
    let testing_match = similar_prompts.iter()
        .find(|p| p.subject_name == "Testing" && p.prompt.id == "unit-tests-004");
    assert!(testing_match.is_some());

    Ok(())
}

#[test]
fn test_similarity_case_insensitive() -> Result<()> {
    let (_temp_dir, manager) = setup_similarity_test_directory()?;

    // Test with different casing
    let task = "ANALYZE CODE FOR BUGS AND ERRORS";
    let similar_prompts = manager.find_similar_prompts(task, 0.5);

    // Should still find similar prompts despite case differences
    assert!(!similar_prompts.is_empty());

    Ok(())
}

#[test]
fn test_similarity_with_url_content() -> Result<()> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    // Create a markdown file for URL reference
    let md_content = "Analyze this code for performance bottlenecks and optimization opportunities";
    let md_file_path = prompts_dir.join("performance_analysis.md");
    fs::write(&md_file_path, md_content)?;

    // Create YAML with URL reference
    let test_yaml = format!(r#"name: "URL Test Prompts"
description: "Test similarity with URL references"
subjects:
  - name: "Performance"
    prompts:
      - title: "Performance analysis"
        content: "file://{}"
        score: 0
        id: "perf-url-001"
"#, md_file_path.to_string_lossy());

    let yaml_file_path = prompts_dir.join("url_test.yaml");
    fs::write(&yaml_file_path, test_yaml)?;

    let manager = PromptManager::load_from_directory(&prompts_dir)?;

    // Search for similar content
    let task = "Optimize code for better performance and identify bottlenecks";
    let similar_prompts = manager.find_similar_prompts(task, 0.5);

    // Should find the URL-referenced prompt
    assert!(!similar_prompts.is_empty());
    
    let url_match = similar_prompts.iter()
        .find(|p| p.prompt.id == "perf-url-001");
    assert!(url_match.is_some());

    Ok(())
}

#[test]
fn test_similarity_scoring_accuracy() -> Result<()> {
    let (_temp_dir, manager) = setup_similarity_test_directory()?;

    // Test exact match
    let task = "Review this code for potential bugs, errors, and issues";
    let similar_prompts = manager.find_similar_prompts(task, 0.0);
    
    let exact_match = similar_prompts.iter()
        .find(|p| p.prompt.id == "analyze-bugs-001");
    assert!(exact_match.is_some());
    assert!(exact_match.unwrap().similarity_score > 0.9);

    // Test partial match
    let task = "Check code for bugs";
    let similar_prompts = manager.find_similar_prompts(task, 0.0);
    
    let partial_match = similar_prompts.iter()
        .find(|p| p.prompt.id == "analyze-bugs-001");
    assert!(partial_match.is_some());
    assert!(partial_match.unwrap().similarity_score > 0.4);
    assert!(partial_match.unwrap().similarity_score < 0.9);

    Ok(())
}