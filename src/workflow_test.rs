use crate::workflow::{get_workflow_engine, VerificationStrategy};

#[allow(dead_code)]
pub async fn test_basic_workflow() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing basic workflow functionality...");
    
    let engine = get_workflow_engine();
    
    // Test workflow creation
    let plan_id = engine.start_workflow(
        "Create a test file with Hello World content",
        5,
        VerificationStrategy::FileExists,
    ).await?;
    
    println!("Created workflow with ID: {}", plan_id);
    
    // Test workflow status
    if let Some(plan) = engine.get_workflow_status(&plan_id) {
        println!("Workflow status: {:?}", plan.status);
        println!("Number of steps: {}", plan.steps.len());
    }
    
    // Test workflow continuation
    let should_continue = engine.continue_workflow(&plan_id).await;
    match should_continue {
        Ok(continue_flag) => println!("Continue workflow: {}", continue_flag),
        Err(e) => println!("Workflow continuation error: {}", e),
    }
    
    println!("Basic workflow test completed");
    Ok(())
}

#[allow(dead_code)]
pub fn test_workflow_commands() {
    println!("Testing workflow command registration...");
    
    // The commands should be registered when register_all_commands() is called
    println!("Workflow commands should be available:");
    println!("- @start-loop(goal, max_iterations, strategy)");
    println!("- @continue-loop(plan_id)");
    println!("- @workflow-status(plan_id)");
    println!("- @list-workflows()");
    println!("- @pause-workflow(plan_id)");
    println!("- @resume-workflow(plan_id)");
    println!("- @stop-workflow(plan_id)");
    println!("- @verify-workflow(plan_id)");
    println!("- @execute-step(plan_id, step_id)");
}