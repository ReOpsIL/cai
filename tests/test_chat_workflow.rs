use anyhow::Result;
use tempfile::{tempdir, TempDir};
use std::fs;
use prompt_manager::{PromptManager, Prompt};

/// Mock OpenRouter client for testing without actual API calls
pub struct MockOpenRouterClient {
    pub plan_tasks_response: Vec<String>,
    pub improve_prompt_response: String,
}

impl MockOpenRouterClient {
    pub fn new() -> Self {
        Self {
            plan_tasks_response: vec![
                "Analyze the application for performance issues".to_string(),
                "Implement caching mechanisms".to_string(),
                "Optimize database queries".to_string(),
            ],
            improve_prompt_response: "Analyze this application thoroughly for performance bottlenecks and optimization opportunities, including database queries, caching strategies, and code efficiency".to_string(),
        }
    }

    pub fn plan_tasks(&self, _user_request: &str) -> Result<Vec<String>> {
        Ok(self.plan_tasks_response.clone())
    }

    pub fn improve_prompt(&self, _original: &str, _new_task: &str) -> Result<String> {
        Ok(self.improve_prompt_response.clone())
    }
}

/// Mock chat interface for testing the workflow logic
pub struct MockChatInterface {
    client: MockOpenRouterClient,
}

impl MockChatInterface {
    pub fn new() -> Self {
        Self {
            client: MockOpenRouterClient::new(),
        }
    }

    /// Simulate the task processing workflow
    pub fn process_task(&self, task: &str, manager: &mut PromptManager) -> Result<TaskProcessingResult> {
        // Find similar prompts (threshold: 0.482 for testing)
        let similar_prompts = manager.find_similar_prompts(task, 0.482);

        if similar_prompts.is_empty() {
            // No similar prompts found - add as new prompt
            self.add_new_prompt(task, manager)?;
            Ok(TaskProcessingResult::NewPromptAdded)
        } else {
            let best_match = &similar_prompts[0];
            
            if best_match.similarity_score >= 0.7 {
                // Very similar prompt exists - increment score
                manager.increment_prompt_score(
                    &best_match.file_name,
                    &best_match.subject_name,
                    &best_match.prompt.id,
                )?;
                Ok(TaskProcessingResult::PromptScored)
            } else if best_match.similarity_score >= 0.4 {
                // Similar but could be improved - update existing prompt
                let improved_content = self.client.improve_prompt(
                    &best_match.prompt.get_resolved_content().await.unwrap_or(best_match.prompt.content.clone()),
                    task,
                )?;
                
                manager.update_prompt(
                    &best_match.file_name,
                    &best_match.subject_name,
                    &best_match.prompt.id,
                    improved_content,
                )?;
                Ok(TaskProcessingResult::PromptUpdated)
            } else {
                // Different enough to be a new prompt
                self.add_new_prompt(task, manager)?;
                Ok(TaskProcessingResult::NewPromptAdded)
            }
        }
    }

    fn add_new_prompt(&self, task: &str, manager: &mut PromptManager) -> Result<()> {
        let ai_file = manager.get_or_create_ai_generated_file()?;
        
        // Determine subject based on task content
        let subject_name = self.categorize_task(task);
        
        // Create title from first few words of task
        let title = task.split_whitespace()
            .take(6)
            .collect::<Vec<_>>()
            .join(" ");
        
        let new_prompt = Prompt {
            title: if title.len() > 50 { 
                format!("{}...", &title[..47])
            } else { 
                title 
            },
            content: task.to_string(),
            score: 0,
            id: uuid::Uuid::new_v4().to_string(),
        };

        manager.add_prompt_to_subject(&ai_file, &subject_name, new_prompt)?;
        Ok(())
    }

    fn categorize_task(&self, task: &str) -> String {
        let task_lower = task.to_lowercase();
        
        if task_lower.contains("performance") || task_lower.contains("optimize") || task_lower.contains("cache") {
            "Performance".to_string()
        } else if task_lower.contains("test") || task_lower.contains("plan") {
            "Testing".to_string()
        } else if task_lower.contains("analyze") || task_lower.contains("review") {
            "Analysis".to_string()
        } else {
            "General".to_string()
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TaskProcessingResult {
    NewPromptAdded,
    PromptUpdated,
    PromptScored,
}

/// Setup test directory with existing prompts for workflow testing
fn setup_workflow_test() -> Result<(TempDir, PromptManager)> {
    let temp_dir = tempdir()?;
    let prompts_dir = temp_dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir)?;

    let test_yaml = r#"name: "Workflow Test"
description: "Test chat workflow functionality"
subjects:
  - name: "Performance"
    prompts:
      - title: "Performance analysis"
        content: "Analyze application performance issues"
        score: 3
        id: "perf-analysis-001"
      - title: "Optimization strategies"
        content: "Implement optimization for better performance"
        score: 1
        id: "optimization-002"
  - name: "Analysis"
    prompts:
      - title: "Code analysis"
        content: "Analyze code for potential issues"
        score: 2
        id: "code-analysis-003"
"#;

    let test_file_path = prompts_dir.join("workflow_test.yaml");
    fs::write(&test_file_path, test_yaml)?;

    let manager = PromptManager::load_from_directory(&prompts_dir)?;
    Ok((temp_dir, manager))
}

#[test]
fn test_new_prompt_addition_workflow() -> Result<()> {
    let (_temp_dir, mut manager) = setup_workflow_test()?;
    let chat = MockChatInterface::new();

    // Process a completely new task
    let task = "Create automated deployment scripts for continuous integration";
    let result = chat.process_task(task, &mut manager)?;

    assert_eq!(result, TaskProcessingResult::NewPromptAdded);

    // Verify AI generated file was created
    let ai_file = manager.get_by_file_name("ai_generated").unwrap();
    assert_eq!(ai_file.prompt_file.name, "AI Generated");

    // Verify the new prompt was added
    let general_subject = ai_file.prompt_file.subjects
        .iter()
        .find(|s| s.name == "General");
    assert!(general_subject.is_some());

    let new_prompt = &general_subject.unwrap().prompts[0];
    assert_eq!(new_prompt.content, task);
    assert_eq!(new_prompt.score, 0);

    Ok(())
}

#[test]
fn test_prompt_scoring_workflow() -> Result<()> {
    let (_temp_dir, mut manager) = setup_workflow_test()?;
    let chat = MockChatInterface::new();

    // Process a task very similar to existing prompt  
    let task = "Analyze application performance issues and bottlenecks";
    let result = chat.process_task(task, &mut manager)?;

    assert_eq!(result, TaskProcessingResult::PromptScored);

    // Verify the existing prompt score was incremented
    let workflow_file = manager.get_by_file_name("workflow_test").unwrap();
    let perf_subject = workflow_file.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Performance")
        .unwrap();

    let scored_prompt = perf_subject.prompts
        .iter()
        .find(|p| p.id == "perf-analysis-001")
        .unwrap();

    assert_eq!(scored_prompt.score, 4); // Should be incremented from 3 to 4

    Ok(())
}

#[test]
fn test_prompt_update_workflow() -> Result<()> {
    let (_temp_dir, mut manager) = setup_workflow_test()?;
    let chat = MockChatInterface::new();

    // Process a task moderately similar to existing prompt
    let task = "Optimize application for improved performance and efficiency";
    let result = chat.process_task(task, &mut manager)?;

    assert_eq!(result, TaskProcessingResult::PromptUpdated);

    // Verify the existing prompt was updated
    let workflow_file = manager.get_by_file_name("workflow_test").unwrap();
    let perf_subject = workflow_file.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Performance")
        .unwrap();

    let updated_prompt = perf_subject.prompts
        .iter()
        .find(|p| p.id == "optimization-002")
        .unwrap();

    // Content should be updated with improved version
    assert_eq!(updated_prompt.content, chat.client.improve_prompt_response);
    assert_eq!(updated_prompt.score, 1); // Score should remain unchanged

    Ok(())
}

#[test]
fn test_multiple_task_processing() -> Result<()> {
    let (_temp_dir, mut manager) = setup_workflow_test()?;
    let chat = MockChatInterface::new();

    let tasks = vec![
        "Analyze application performance issues and bottlenecks", // Should score existing
        "Create unit tests for all modules",                      // Should add new
        "Optimize application for better performance",            // Should update existing
    ];

    let mut results = Vec::new();
    for task in &tasks {
        let result = chat.process_task(task, &mut manager)?;
        results.push(result);
    }

    // Verify expected results
    assert_eq!(results[0], TaskProcessingResult::PromptScored);
    assert_eq!(results[1], TaskProcessingResult::NewPromptAdded);
    assert_eq!(results[2], TaskProcessingResult::PromptUpdated);

    // Verify AI generated file has new prompts
    let ai_file = manager.get_by_file_name("ai_generated").unwrap();
    assert!(ai_file.prompt_file.subjects.len() > 0);

    Ok(())
}

#[test]
fn test_task_categorization() -> Result<()> {
    let (_temp_dir, mut manager) = setup_workflow_test()?;
    let chat = MockChatInterface::new();

    // Test performance categorization
    let perf_task = "Optimize application performance and caching strategies";
    chat.process_task(perf_task, &mut manager)?;

    // Test testing categorization  
    let test_task = "Plan comprehensive testing strategy for the application";
    chat.process_task(test_task, &mut manager)?;

    // Test analysis categorization
    let analysis_task = "Analyze code structure and architectural patterns";
    chat.process_task(analysis_task, &mut manager)?;

    // Verify tasks were categorized correctly
    let ai_file = manager.get_by_file_name("ai_generated").unwrap();
    
    let performance_subject = ai_file.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Performance");
    assert!(performance_subject.is_some());

    let testing_subject = ai_file.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Testing");
    assert!(testing_subject.is_some());

    let analysis_subject = ai_file.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Analysis");
    assert!(analysis_subject.is_some());

    Ok(())
}

#[test]
fn test_workflow_persistence() -> Result<()> {
    let (temp_dir, mut manager) = setup_workflow_test()?;
    let chat = MockChatInterface::new();

    // Process tasks that modify the repository
    chat.process_task("Analyze application performance issues and bottlenecks", &mut manager)?;
    chat.process_task("Create deployment automation", &mut manager)?;

    // Reload manager from disk
    let reloaded_manager = PromptManager::load_from_directory(temp_dir.path().join("prompts"))?;

    // Verify scored prompt persisted
    let workflow_file = reloaded_manager.get_by_file_name("workflow_test").unwrap();
    let perf_prompt = workflow_file.prompt_file.subjects
        .iter()
        .find(|s| s.name == "Performance")
        .unwrap()
        .prompts
        .iter()
        .find(|p| p.id == "perf-analysis-001")
        .unwrap();
    assert_eq!(perf_prompt.score, 4);

    // Verify new prompt persisted
    let ai_file = reloaded_manager.get_by_file_name("ai_generated");
    assert!(ai_file.is_some());

    Ok(())
}

#[test]
fn test_similarity_threshold_boundaries() -> Result<()> {
    let (_temp_dir, mut manager) = setup_workflow_test()?;
    let chat = MockChatInterface::new();

    // Task with similarity just above scoring threshold (≥0.7)
    let high_sim_task = "Analyze application performance issues and bottlenecks";
    let result = chat.process_task(high_sim_task, &mut manager)?;
    assert_eq!(result, TaskProcessingResult::PromptScored);

    // Task with similarity in update range (0.4-0.7)
    let med_sim_task = "Optimize application for improved performance";
    let result = chat.process_task(med_sim_task, &mut manager)?;
    assert_eq!(result, TaskProcessingResult::PromptUpdated);

    // Task with similarity below update threshold (<0.4)
    let low_sim_task = "Design user interface mockups and wireframes";
    let result = chat.process_task(low_sim_task, &mut manager)?;
    assert_eq!(result, TaskProcessingResult::NewPromptAdded);

    Ok(())
}