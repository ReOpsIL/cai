use crate::logger::{log_debug, log_error, log_info, log_warn, ops};
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::time::Instant;
use std::collections::HashSet;
use crate::mcp_manager;

#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub parameters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelection {
    pub tool_name: String,
    pub execution_order: u32,
    pub parameters: HashMap<String, Value>,
    pub rationale: String,
}

#[derive(Debug, Deserialize)]
struct ToolAnalysisResponse {
    selected_tools: Vec<ToolSelection>,
    execution_plan: String,
    expected_outcome: String,
}

#[derive(Debug)]
pub struct OpenRouterClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenRouterClient {
    pub async fn new() -> Result<Self> {
        log_info!("openrouter", "🌍 Initializing OpenRouter client");
        let init_start = Instant::now();

        let api_key = env::var("OPENROUTER_API_KEY")
            .context("OPENROUTER_API_KEY environment variable not set")?;
        // Do not log API key presence/length

        let timeout_secs: u64 = env::var("CAI_HTTP_TIMEOUT").ok().and_then(|s| s.parse().ok()).unwrap_or(60);
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .context("Failed to create HTTP client")?;

        let init_duration = init_start.elapsed().as_millis() as u64;
        ops::performance("OPENROUTER_INIT", init_duration);
        log_info!(
            "openrouter",
            "✅ OpenRouter client initialized successfully"
        );

        let base_url = env::var("CAI_API_BASE").unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());
        Ok(Self { client, api_key, base_url })
    }

    pub async fn chat_completion(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let completion_start = Instant::now();
        log_debug!(
            "openrouter",
            "💬 Starting chat completion with {} messages",
            messages.len()
        );

        let model = env::var("CAI_MODEL").unwrap_or_else(|_| "google/gemini-2.5-flash".to_string());
        log_debug!("openrouter", "🤖 Using model: {}", model);

        let request = OpenRouterRequest {
            model,
            messages: messages.clone(),
            max_tokens: Some(4000),
            temperature: Some(0.7),
        };

        for (i, msg) in messages.iter().enumerate() {
            log_debug!(
                "openrouter",
                "📝 Message {}: {} ({})",
                i + 1,
                if msg.content.len() > 100 {
                    format!("{}...", &msg.content[..100])
                } else {
                    msg.content.clone()
                },
                msg.role
            );
        }

        let url = format!("{}/chat/completions", self.base_url);
        log_debug!("openrouter", "🚀 Sending request to: {}", url);
        ops::network_operation("POST", &url, None);

        let request_start = Instant::now();
        // Simple retry with backoff for transient failures
        let mut last_err: Option<anyhow::Error> = None;
        let mut response_opt = None;
        for attempt in 0..3 {
            let resp_res = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .header("X-Title", "CAI Prompt Manager")
                .json(&request)
                .send()
                .await;
            match resp_res {
                Ok(resp) => {
                    response_opt = Some(resp);
                    break;
                }
                Err(e) => {
                    last_err = Some(anyhow::anyhow!(e));
                    if attempt < 2 {
                        let delay = 200 * (attempt + 1) as u64;
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        let response = response_opt.ok_or_else(|| last_err.unwrap_or_else(|| anyhow::anyhow!("OpenRouter request failed")))?;

        let request_duration = request_start.elapsed().as_millis() as u64;
        ops::performance("HTTP_REQUEST", request_duration);

        let status = response.status();
        let status_code = status.as_u16();
        ops::network_operation("POST", &url, Some(status_code));
        log_debug!("openrouter", "📊 HTTP response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            let error_msg = format!("OpenRouter API error {}: {}", status, error_text);
            ops::error_with_context("OPENROUTER_API", &error_msg, Some(&url));
            log_error!("openrouter", "❌ API request failed: {}", error_msg);
            return Err(anyhow::anyhow!(error_msg));
        }

        let parse_start = Instant::now();
        let openrouter_response: OpenRouterResponse = response
            .json()
            .await
            .context("Failed to parse OpenRouter response")?;
        let parse_duration = parse_start.elapsed().as_millis() as u64;
        ops::performance("JSON_PARSE", parse_duration);

        let completion_duration = completion_start.elapsed().as_millis() as u64;
        ops::performance("CHAT_COMPLETION", completion_duration);

        let result = openrouter_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No response choices received"));

        match &result {
            Ok(content) => {
                log_info!(
                    "openrouter",
                    "✅ Chat completion successful ({} chars, {}ms)",
                    content.len(),
                    completion_duration
                );
                log_debug!(
                    "openrouter",
                    "💬 Response preview: {}",
                    if content.len() > 200 {
                        format!("{}...", &content[..200])
                    } else {
                        content.clone()
                    }
                );
            }
            Err(e) => {
                ops::error_with_context("OPENROUTER_RESPONSE", &e.to_string(), None);
                log_error!("openrouter", "❌ Chat completion failed: {}", e);
            }
        }

        result
    }

    pub async fn plan_tasks(&self, user_request: &str) -> Result<Vec<String>> {
        let planning_start = Instant::now();
        log_info!("openrouter", "📋 Planning tasks for user request (MCP-aware)");
        log_debug!("openrouter", "📥 User request: {}", user_request);

        // Discover available MCP tools dynamically via MCP Manager
        let mut available_tool_names: Vec<String> = Vec::new();
        let mut available_tool_set: HashSet<String> = HashSet::new();
        {
            let global_manager = mcp_manager::get_mcp_manager();
            let guard = global_manager.lock().await;
            if let Some(manager) = guard.as_ref() {
                let servers = manager.list_active_servers().await;
                log_debug!("openrouter", "📡 Found {} active MCP server(s)", servers.len());
                for server in servers {
                    match manager.list_tools(&server).await {
                        Ok(mut tools) => {
                            tools.sort();
                            for t in tools {
                                available_tool_set.insert(t.clone());
                                available_tool_names.push(t);
                            }
                        }
                        Err(e) => {
                            log_warn!("openrouter", "⚠️ Failed to list tools from server '{}': {}", server, e);
                        }
                    }
                }
            } else {
                log_warn!("openrouter", "⚠️ MCP manager not initialized; proceeding without tool discovery");
            }
        }

        // Build the planning instruction with strict JSON schema and the discovered tools
        let tools_section = if available_tool_names.is_empty() {
            "(no tools discovered; if tools are required, still produce tasks but mark mcp_tool as 'UNAVAILABLE')".to_string()
        } else {
            format!("Available MCP tools (use exact names): {}", available_tool_names.join(", "))
        };

        #[derive(Debug, Deserialize)]
        struct PlannedTask {
            id: String,
            #[serde(default)]
            title: Option<String>,
            description: String,
            mcp_tool: String,
            #[serde(default)]
            params: Option<HashMap<String, Value>>,
            #[serde(default)]
            depends_on: Option<Vec<String>>,
        }

        // The system prompt forces a JSON array of PlannedTask objects
        let system_prompt = format!(
            r#"You are a coding CLI planner. Produce an execution-ready, deeply granular plan as a JSON array of task objects only. Each task must be atomic, action-oriented, and explicitly call a single MCP tool from the discovered set.

Requirements:
- Use the discovered tools exactly as named.
- Include sufficient technical detail in 'description' and concrete 'params' for the tool call.
- Ensure logical ordering and explicit dependencies via 'depends_on' (list of task ids).
- IDs must be short, unique strings (e.g., t1, t2...).
- Output ONLY raw JSON array with the following schema for each task:
  {{
    "id": "t1",
    "title": "short title",
    "description": "precise action to perform",
    "mcp_tool": "exact_tool_name",
    "params": {{ "k": "v" }},
    "depends_on": ["t_prev"]
  }}

Context:
- {tools_section}
- User request: {user_request}
"#
        );

        let messages = vec![
            ChatMessage { role: "system".to_string(), content: system_prompt },
        ];

        log_debug!("openrouter", "🚀 Sending MCP-aware planning request to OpenRouter");
        let response = self.chat_completion(messages).await?;
        log_debug!("openrouter", "💬 Received response for task planning");

        // Extract potential JSON (handle code fences)
        let json_str = self.extract_json_from_response(&response);

        // Parse as array or object with tasks field
        let parsed_value: Value = serde_json::from_str(&json_str)
            .unwrap_or(Value::Null);

        let mut planned_tasks: Vec<PlannedTask> = Vec::new();
        if let Value::Array(items) = parsed_value {
            for item in items {
                if let Ok(task) = serde_json::from_value::<PlannedTask>(item) {
                    planned_tasks.push(task);
                }
            }
        } else if let Value::Object(map) = parsed_value {
            if let Some(tasks_val) = map.get("tasks") {
                if let Ok(items) = serde_json::from_value::<Vec<PlannedTask>>(tasks_val.clone()) {
                    planned_tasks = items;
                }
            }
        }

        // Fallback to numbered lines parsing if JSON not obtained
        if planned_tasks.is_empty() {
            log_warn!("openrouter", "⚠️ JSON tasks not found; falling back to numbered lines parsing");
            let tasks: Vec<String> = response
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    if let Some(pos) = line.find(". ") {
                        let number_part = &line[..pos];
                        if number_part.chars().all(|c| c.is_ascii_digit()) {
                            return Some(line[pos + 2..].trim().to_string());
                        }
                    }
                    None
                })
                .collect();
            if tasks.is_empty() {
                return Ok(vec![response.trim().to_string()]);
            } else {
                return Ok(tasks);
            }
        }

        // Validate tool names; mark unavailable tools
        for t in &mut planned_tasks {
            if !available_tool_set.is_empty() && !available_tool_set.contains(&t.mcp_tool) {
                t.mcp_tool = format!("{}{}", t.mcp_tool, " (UNAVAILABLE)");
            }
        }

        // Topologically sort tasks by depends_on
        let mut id_to_task: HashMap<String, PlannedTask> = HashMap::new();
        for t in planned_tasks.into_iter() {
            id_to_task.insert(t.id.clone(), t);
        }
        let mut incoming_count: HashMap<String, usize> = HashMap::new();
        for (id, t) in &id_to_task {
            let deps = t.depends_on.as_ref().map(|v| v.len()).unwrap_or(0);
            incoming_count.insert(id.clone(), deps);
        }
        let mut ready: Vec<String> = incoming_count
            .iter()
            .filter_map(|(id, &c)| if c == 0 { Some(id.clone()) } else { None })
            .collect();
        ready.sort();
        let mut ordered_ids: Vec<String> = Vec::new();
        while let Some(id) = ready.pop() {
            ordered_ids.push(id.clone());
            // Decrease incoming counts for dependents
            for (other_id, other_task) in &id_to_task {
                if let Some(deps) = &other_task.depends_on {
                    if deps.iter().any(|d| d == &id) {
                        if let Some(cnt) = incoming_count.get_mut(other_id) {
                            if *cnt > 0 { *cnt -= 1; }
                            if *cnt == 0 && !ordered_ids.contains(other_id) && !ready.contains(other_id) {
                                ready.push(other_id.clone());
                            }
                        }
                    }
                }
            }
        }
        if ordered_ids.len() < id_to_task.len() {
            log_warn!("openrouter", "⚠️ Dependency cycle or unresolved dependencies detected; preserving original order");
            ordered_ids = id_to_task.keys().cloned().collect();
            ordered_ids.sort();
        }

        // Convert to execution-ready CLI strings while preserving rich detail
        let mut final_tasks: Vec<String> = Vec::new();
        for id in ordered_ids {
            if let Some(t) = id_to_task.get(&id) {
                let title = t.title.clone().unwrap_or_else(|| "".to_string());
                let params_json = serde_json::to_string(&t.params.as_ref().cloned().unwrap_or_default()).unwrap_or("{}".to_string());
                let deps = t.depends_on.as_ref().map(|v| v.join(", ")).unwrap_or_else(|| "".to_string());
                let line = format!(
                    "TASK id={id}; title=\"{title}\"; tool={tool}; params={params}; depends_on=[{deps}]; action=\"{desc}\"",
                    id = t.id,
                    title = title.replace('"', "'"),
                    tool = t.mcp_tool,
                    params = params_json,
                    deps = deps,
                    desc = t.description.replace('"', "'"),
                );
                final_tasks.push(line);
            }
        }

        let planning_duration = planning_start.elapsed().as_millis() as u64;
        ops::performance("TASK_PLANNING", planning_duration);
        for (i, task) in final_tasks.iter().enumerate() {
            log_debug!("openrouter", "📝 Task {}: {}", i + 1, task);
        }
        log_info!("openrouter", "✅ Task planning completed: {} tasks generated", final_tasks.len());
        Ok(final_tasks)
    }

    pub async fn improve_prompt(&self, original_prompt: &str, new_task: &str) -> Result<String> {
        let improve_start = Instant::now();
        log_info!("openrouter", "🔄 Improving prompt with new task input");
        log_debug!(
            "openrouter",
            "📝 Original prompt: {}",
            if original_prompt.len() > 100 {
                format!("{}...", &original_prompt[..100])
            } else {
                original_prompt.to_string()
            }
        );
        log_debug!("openrouter", "✨ New task: {}", new_task);

        let prompt = format!(
            r#"You have an existing prompt and a new task that are similar. Your job is to create an improved version that combines the best aspects of both.

Guidelines:
1. Merge the intent and scope of both prompts
2. Make the result more comprehensive and actionable
3. Ensure clarity and specificity
4. Remove redundancy
5. Keep the tone consistent

Existing prompt: {}

New task: {}

Improved prompt:"#,
            original_prompt, new_task
        );

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        log_debug!(
            "openrouter",
            "🚀 Sending prompt improvement request to OpenRouter"
        );
        let result = self.chat_completion(messages).await;

        let improve_duration = improve_start.elapsed().as_millis() as u64;
        ops::performance("PROMPT_IMPROVEMENT", improve_duration);

        match &result {
            Ok(improved) => {
                log_info!(
                    "openrouter",
                    "✅ Prompt improvement completed ({} chars, {}ms)",
                    improved.len(),
                    improve_duration
                );
                log_debug!(
                    "openrouter",
                    "🎆 Improved prompt: {}",
                    if improved.len() > 200 {
                        format!("{}...", &improved[..200])
                    } else {
                        improved.clone()
                    }
                );
            }
            Err(e) => {
                ops::error_with_context("PROMPT_IMPROVEMENT", &e.to_string(), None);
                log_error!("openrouter", "❌ Prompt improvement failed: {}", e);
            }
        }

        result
    }

    /// Analyze a task and determine which MCP tools should be used
    pub async fn analyze_task_for_tools(&self, task_description: &str, available_tools: &[ToolMetadata]) -> Result<Vec<ToolSelection>> {
        let analysis_start = Instant::now();
        log_info!("openrouter", "🔍 Analyzing task for MCP tool selection");
        log_debug!("openrouter", "📋 Task: {}", task_description);
        log_debug!("openrouter", "🔧 Available tools: {}", available_tools.len());

        // Build tools description for the prompt
        let tools_description = available_tools.iter()
            .map(|tool| format!("- **{}**: {} (Parameters: {})", 
                               tool.name, 
                               tool.description,
                               tool.parameters.join(", ")))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            r#"You are an intelligent tool dispatcher for a Model Context Protocol (MCP) system. Your task is to analyze a user request and determine which tools from the available toolkit should be executed to fulfill the request effectively.

## Available Tools:
{}

## User Task:
{}

## Analysis Framework:
Evaluate the task based on:
1. **Task Requirements**: What specific actions or data are needed?
2. **Tool Capabilities**: Which tools can address each requirement?
3. **Execution Dependencies**: Are there tools that must run before others?
4. **Data Flow**: How will outputs from one tool serve as inputs to another?
5. **Efficiency**: What's the minimal set of tools needed to complete the task?

## Response Format:
Respond with ONLY a valid JSON object in this exact format:

```json
{{
  "selected_tools": [
    {{
      "tool_name": "exact_tool_identifier",
      "execution_order": 1,
      "parameters": {{
        "param1": "value1",
        "param2": "value2"
      }},
      "rationale": "Brief explanation why this tool is needed"
    }}
  ],
  "execution_plan": "Brief description of how tools will work together",
  "expected_outcome": "What the combined tool execution should achieve"
}}
```

Important:
- Only select tools that are listed in the available tools
- Use exact tool names as provided
- Assign execution_order starting from 1
- Include realistic parameter values based on the task
- If no tools are suitable, return an empty selected_tools array

JSON Response:"#,
            tools_description, task_description
        );

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        log_debug!("openrouter", "🚀 Sending tool analysis request to OpenRouter");
        let response = self.chat_completion(messages).await?;

        let analysis_duration = analysis_start.elapsed().as_millis() as u64;
        ops::performance("TOOL_ANALYSIS", analysis_duration);

        log_debug!("openrouter", "📄 Raw LLM response: {}", response);

        // Try to extract JSON from the response (LLMs sometimes wrap JSON in markdown)
        let json_str = self.extract_json_from_response(&response);
        log_debug!("openrouter", "🔧 Extracted JSON: {}", json_str);

        // Parse the JSON response
        let parsed_response: ToolAnalysisResponse = serde_json::from_str(&json_str)
            .context(format!("Failed to parse LLM tool analysis response as JSON. Response: {}", json_str))?;

        log_info!("openrouter", "✅ Tool analysis completed ({} tools selected, {}ms)", 
                 parsed_response.selected_tools.len(), analysis_duration);

        Ok(parsed_response.selected_tools)
    }

    /// Extract JSON from LLM response, handling markdown code blocks and other formatting
    fn extract_json_from_response(&self, response: &str) -> String {
        // Try to find JSON within markdown code blocks
        if let Some(start) = response.find("```json") {
            if let Some(end) = response[start..].find("```") {
                let json_start = start + 7; // Length of "```json"
                let json_end = start + end;
                if json_start < json_end {
                    return response[json_start..json_end].trim().to_string();
                }
            }
        }

        // Try to find JSON within plain code blocks
        if let Some(start) = response.find("```") {
            if let Some(end) = response[start + 3..].find("```") {
                let json_start = start + 3;
                let json_end = start + 3 + end;
                if json_start < json_end {
                    let potential_json = response[json_start..json_end].trim();
                    if (potential_json.starts_with('{') && potential_json.ends_with('}')) ||
                       (potential_json.starts_with('[') && potential_json.ends_with(']')) {
                        return potential_json.to_string();
                    }
                }
            }
        }

        // Try to find JSON by looking for { } or [ ] boundaries
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                if start < end {
                    return response[start..=end].trim().to_string();
                }
            }
        }
        
        // Try to find JSON array by looking for [ ] boundaries
        if let Some(start) = response.find('[') {
            if let Some(end) = response.rfind(']') {
                if start < end {
                    return response[start..=end].trim().to_string();
                }
            }
        }

        // Return the original response if no JSON structure found
        response.trim().to_string()
    }
}
