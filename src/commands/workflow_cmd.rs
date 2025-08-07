use crate::commands_registry::{register_command, Command, CommandType};
use crate::workflow::{get_workflow_engine, VerificationStrategy};
use regex::Regex;

pub fn register_workflow_commands() {
    // Start workflow command
    register_command(Command {
        name: "start-loop".to_string(),
        pattern: Regex::new(r"@start-loop\(\s*(.+?)\s*,\s*(\d+)\s*,\s*(\w+)\s*\)").unwrap(),
        description: "Start an autonomous workflow loop".to_string(),
        usage_example: "@start-loop(Set up Rust project, 10, file_exists)".to_string(),
        handler: |params| {
            if params.len() < 3 {
                return Ok(Some("Usage: @start-loop(goal, max_iterations, verification_strategy)".to_string()));
            }

            let goal = &params[0];
            let max_iterations: usize = params[1].parse().unwrap_or(10);
            let verification_strategy = match params[2].as_str() {
                "file_exists" => VerificationStrategy::FileExists,
                "command_success" => VerificationStrategy::CommandSuccess,
                "llm_validation" => VerificationStrategy::LLMValidation,
                "combined" => VerificationStrategy::Combined,
                pattern => VerificationStrategy::OutputPattern(pattern.to_string()),
            };

            let engine = get_workflow_engine();
            let runtime = tokio::runtime::Handle::current();

            match runtime.block_on(engine.start_workflow(goal, max_iterations, verification_strategy)) {
                Ok(plan_id) => {
                    Ok(Some(format!(
                        "Started workflow: {}\nGoal: {}\nMax iterations: {}\nPlan ID: {}",
                        plan_id, goal, max_iterations, plan_id
                    )))
                }
                Err(e) => Ok(Some(format!("Failed to start workflow: {}", e))),
            }
        },
        section: "workflow".to_string(),
        command_type: CommandType::LLM,
        autocomplete_handler: None,
    });

    // Continue workflow command
    register_command(Command {
        name: "continue-loop".to_string(),
        pattern: Regex::new(r"@continue-loop\(\s*(\S+)\s*\)").unwrap(),
        description: "Continue executing workflow steps".to_string(),
        usage_example: "@continue-loop(plan_id)".to_string(),
        handler: |params| {
            if params.is_empty() {
                return Ok(Some("Usage: @continue-loop(plan_id)".to_string()));
            }

            let plan_id = &params[0];
            let engine = get_workflow_engine();
            let runtime = tokio::runtime::Handle::current();

            match runtime.block_on(engine.continue_workflow(plan_id)) {
                Ok(should_continue) => {
                    if should_continue {
                        Ok(Some(format!("Workflow {} continued. Execute @continue-loop({}) for next iteration.", plan_id, plan_id)))
                    } else {
                        Ok(Some(format!("Workflow {} completed successfully!", plan_id)))
                    }
                }
                Err(e) => Ok(Some(format!("Failed to continue workflow: {}", e))),
            }
        },
        section: "workflow".to_string(),
        command_type: CommandType::LLM,
        autocomplete_handler: None,
    });

    // Workflow status command
    register_command(Command {
        name: "workflow-status".to_string(),
        pattern: Regex::new(r"@workflow-status\(\s*(\S+)\s*\)").unwrap(),
        description: "Get status of a workflow".to_string(),
        usage_example: "@workflow-status(plan_id)".to_string(),
        handler: |params| {
            if params.is_empty() {
                return Ok(Some("Usage: @workflow-status(plan_id)".to_string()));
            }

            let plan_id = &params[0];
            let engine = get_workflow_engine();

            match engine.get_workflow_status(plan_id) {
                Some(plan) => {
                    let status_text = format!(
                        "Workflow: {}\nGoal: {}\nStatus: {:?}\nProgress: {}/{} steps\nIteration: {}/{}\nSteps:\n{}",
                        plan.id,
                        plan.goal,
                        plan.status,
                        plan.current_step,
                        plan.steps.len(),
                        plan.current_iteration,
                        plan.max_iterations,
                        plan.steps
                            .iter()
                            .enumerate()
                            .map(|(i, step)| format!(
                                "  {}. {} - {:?}{}",
                                i + 1,
                                step.description,
                                step.status,
                                if let Some(result) = &step.result {
                                    format!(" ({})", result)
                                } else {
                                    String::new()
                                }
                            ))
                            .collect::<Vec<_>>()
                            .join("\n")
                    );
                    Ok(Some(status_text))
                }
                None => Ok(Some(format!("Workflow {} not found", plan_id))),
            }
        },
        section: "workflow".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // List workflows command
    register_command(Command {
        name: "list-workflows".to_string(),
        pattern: Regex::new(r"@list-workflows\(\s*\)").unwrap(),
        description: "List all active workflows".to_string(),
        usage_example: "@list-workflows()".to_string(),
        handler: |_| {
            let engine = get_workflow_engine();
            let workflow_ids = engine.list_workflows();

            if workflow_ids.is_empty() {
                Ok(Some("No active workflows".to_string()))
            } else {
                let mut result = "Active workflows:\n".to_string();
                for (i, id) in workflow_ids.iter().enumerate() {
                    if let Some(plan) = engine.get_workflow_status(id) {
                        result.push_str(&format!(
                            "{}. {} - {} ({:?})\n",
                            i + 1,
                            id,
                            plan.goal,
                            plan.status
                        ));
                    }
                }
                Ok(Some(result))
            }
        },
        section: "workflow".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Pause workflow command
    register_command(Command {
        name: "pause-workflow".to_string(),
        pattern: Regex::new(r"@pause-workflow\(\s*(\S+)\s*\)").unwrap(),
        description: "Pause a running workflow".to_string(),
        usage_example: "@pause-workflow(plan_id)".to_string(),
        handler: |params| {
            if params.is_empty() {
                return Ok(Some("Usage: @pause-workflow(plan_id)".to_string()));
            }

            let plan_id = &params[0];
            let engine = get_workflow_engine();

            match engine.pause_workflow(plan_id) {
                Ok(()) => Ok(Some(format!("Workflow {} paused", plan_id))),
                Err(e) => Ok(Some(format!("Failed to pause workflow: {}", e))),
            }
        },
        section: "workflow".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Resume workflow command
    register_command(Command {
        name: "resume-workflow".to_string(),
        pattern: Regex::new(r"@resume-workflow\(\s*(\S+)\s*\)").unwrap(),
        description: "Resume a paused workflow".to_string(),
        usage_example: "@resume-workflow(plan_id)".to_string(),
        handler: |params| {
            if params.is_empty() {
                return Ok(Some("Usage: @resume-workflow(plan_id)".to_string()));
            }

            let plan_id = &params[0];
            let engine = get_workflow_engine();

            match engine.resume_workflow(plan_id) {
                Ok(()) => Ok(Some(format!("Workflow {} resumed", plan_id))),
                Err(e) => Ok(Some(format!("Failed to resume workflow: {}", e))),
            }
        },
        section: "workflow".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Stop workflow command
    register_command(Command {
        name: "stop-workflow".to_string(),
        pattern: Regex::new(r"@stop-workflow\(\s*(\S+)\s*\)").unwrap(),
        description: "Stop a workflow".to_string(),
        usage_example: "@stop-workflow(plan_id)".to_string(),
        handler: |params| {
            if params.is_empty() {
                return Ok(Some("Usage: @stop-workflow(plan_id)".to_string()));
            }

            let plan_id = &params[0];
            let engine = get_workflow_engine();

            match engine.stop_workflow(plan_id) {
                Ok(()) => Ok(Some(format!("Workflow {} stopped", plan_id))),
                Err(e) => Ok(Some(format!("Failed to stop workflow: {}", e))),
            }
        },
        section: "workflow".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Verify workflow command
    register_command(Command {
        name: "verify-workflow".to_string(),
        pattern: Regex::new(r"@verify-workflow\(\s*(\S+)\s*\)").unwrap(),
        description: "Verify workflow progress and success".to_string(),
        usage_example: "@verify-workflow(plan_id)".to_string(),
        handler: |params| {
            if params.is_empty() {
                return Ok(Some("Usage: @verify-workflow(plan_id)".to_string()));
            }

            let plan_id = &params[0];
            let engine = get_workflow_engine();
            let runtime = tokio::runtime::Handle::current();

            match runtime.block_on(engine.verify_progress(plan_id)) {
                Ok(result) => {
                    Ok(Some(format!(
                        "Verification result for {}:\nSuccess: {}\nScore: {:.1}%\nMessage: {}",
                        plan_id,
                        result.success,
                        result.score * 100.0,
                        result.message
                    )))
                }
                Err(e) => Ok(Some(format!("Failed to verify workflow: {}", e))),
            }
        },
        section: "workflow".to_string(),
        command_type: CommandType::LLM,
        autocomplete_handler: None,
    });

    // Execute specific step command
    register_command(Command {
        name: "execute-step".to_string(),
        pattern: Regex::new(r"@execute-step\(\s*(\S+)\s*,\s*(\S+)\s*\)").unwrap(),
        description: "Execute a specific workflow step".to_string(),
        usage_example: "@execute-step(plan_id, step_id)".to_string(),
        handler: |params| {
            if params.len() < 2 {
                return Ok(Some("Usage: @execute-step(plan_id, step_id)".to_string()));
            }

            let plan_id = &params[0];
            let step_id = &params[1];
            let engine = get_workflow_engine();
            let runtime = tokio::runtime::Handle::current();

            match runtime.block_on(engine.execute_step(plan_id, step_id)) {
                Ok(result) => {
                    Ok(Some(format!(
                        "Step {} execution result:\nSuccess: {}\nOutput: {}\nError: {}",
                        step_id,
                        result.success,
                        result.output.unwrap_or_else(|| "None".to_string()),
                        result.error.unwrap_or_else(|| "None".to_string())
                    )))
                }
                Err(e) => Ok(Some(format!("Failed to execute step: {}", e))),
            }
        },
        section: "workflow".to_string(),
        command_type: CommandType::LLM,
        autocomplete_handler: None,
    });
}