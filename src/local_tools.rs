use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use glob::glob;
use regex::Regex;
use walkdir::WalkDir;
use crate::task_state::get_global_task_state;

// Special constant server name used to mark built-in tools
pub const LOCAL_SERVER_NAME: &str = "__local__";

// List of built-in tool names exposed by the local tools module
pub fn list_local_tools() -> Vec<String> {
    vec![
        "list_directory".to_string(),
        "read_file".to_string(),
        "write_file".to_string(),
        "edit_file".to_string(),
        "delete_path".to_string(),
        "search_files".to_string(),
        "execute_command".to_string(),
        "glob_files".to_string(),
        "web_fetch".to_string(),
        "create_tasks".to_string(),
        "update_tasks".to_string(),
        "download_file".to_string(),
        "multiedit_file".to_string(),
        "read_many_files".to_string(),
        "diff_files".to_string(),
        "batch_file_search".to_string(),
    ]
}

// Execute a local tool by name with JSON arguments; returns a JSON Value similar to MCP response payloads
pub fn execute_local_tool(tool_name: &str, arguments: Value) -> Result<Value> {
    match tool_name {
        "list_directory" => list_directory(arguments),
        "read_file" => read_file(arguments),
        "write_file" => write_file(arguments),
        "edit_file" => edit_file(arguments),
        "delete_path" => delete_path(arguments),
        "search_files" => search_files(arguments),
        "execute_command" => execute_command(arguments),
        "glob_files" => glob_files(arguments),
        "web_fetch" => web_fetch(arguments),
        "create_tasks" => create_tasks(arguments),
        "update_tasks" => update_tasks(arguments),
        "download_file" => download_file(arguments),
        "multiedit_file" => multiedit_file(arguments),
        "read_many_files" => read_many_files(arguments),
        "diff_files" => diff_files(arguments),
        "batch_file_search" => batch_file_search(arguments),
        _ => Err(anyhow!(format!("Unknown local tool: {}", tool_name))),
    }
}

fn to_abs(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        let full_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(p);
        
        // Try to canonicalize, but if it fails (e.g., file doesn't exist yet),
        // return the joined path without canonicalization
        full_path.canonicalize().unwrap_or(full_path)
    }
}

fn list_directory(args: Value) -> Result<Value> {
    let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'path'"))?;
    let dir = to_abs(path);
    if !dir.exists() {
        return Err(anyhow!("directory not found"));
    }
    if !dir.is_dir() {
        return Err(anyhow!("path is not a directory"));
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        let file_type = meta.file_type();
        entries.push(json!({
            "name": entry.file_name().to_string_lossy(),
            "path": entry.path().to_string_lossy(),
            "isDirectory": file_type.is_dir(),
            "size": if file_type.is_file() { meta.len() } else { 0 },
        }));
    }
    Ok(json!({"ok": true, "entries": entries}))
}

fn read_file(args: Value) -> Result<Value> {
    let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'path'"))?;
    let start_line = args.get("start_line").and_then(|v| v.as_u64()).map(|x| x as usize);
    let end_line = args.get("end_line").and_then(|v| v.as_u64()).map(|x| x as usize);
    
    let p = to_abs(path);
    if !p.exists() { return Err(anyhow!("file not found")); }
    if !p.is_file() { return Err(anyhow!("path is not a file")); }

    // Basic size limit to prevent huge reads (50MB)
    let meta = fs::metadata(&p)?;
    if meta.len() > 50 * 1024 * 1024 { return Err(anyhow!("file too large (>50MB)")); }

    let mut file = fs::File::open(&p)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
    // SAFETY: Record file read for read-before-edit validation
    let safety_validator = crate::tool_safety::get_global_safety_validator();
    safety_validator.record_file_read(&p);
    
    // Handle line range if specified
    if let Some(start) = start_line {
        let lines: Vec<&str> = content.lines().collect();
        let start_idx = start.saturating_sub(1); // Convert to 0-indexed
        let end_idx = end_line.unwrap_or(lines.len()).min(lines.len());
        
        if start_idx >= lines.len() {
            return Err(anyhow!("start line exceeds file length"));
        }
        
        let selected_lines = &lines[start_idx..end_idx];
        let selected_content = selected_lines.join("\n");
        
        return Ok(json!({
            "ok": true, 
            "content": selected_content,
            "lines_shown": [start, end_idx],
            "total_lines": lines.len()
        }));
    }

    Ok(json!({
        "ok": true, 
        "content": content,
        "total_lines": content.lines().count()
    }))
}

fn write_file(args: Value) -> Result<Value> {
    let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'path'"))?;
    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let overwrite = args.get("overwrite").and_then(|v| v.as_bool()).unwrap_or(true);

    let p = to_abs(path);
    if p.exists() && !overwrite {
        return Err(anyhow!("file already exists; set overwrite=true to replace"));
    }
    
    // SAFETY: Validate path and read-before-edit if file exists
    let safety_validator = crate::tool_safety::get_global_safety_validator();
    if let Err(safety_error) = safety_validator.validate_path(&p) {
        return Err(anyhow!("Safety validation failed: {}", safety_error));
    }
    if p.exists() {
        if let Err(safety_error) = safety_validator.validate_edit_operation(&p) {
            return Err(anyhow!("Safety validation failed: {}", safety_error));
        }
    }

    if let Some(parent) = p.parent() { fs::create_dir_all(parent)?; }
    let mut file = fs::File::create(&p)?;
    file.write_all(content.as_bytes())?;
    Ok(json!({"ok": true, "path": p.to_string_lossy()}))
}

fn edit_file(args: Value) -> Result<Value> {
    let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'path'"))?;
    let old_text = args.get("old_text").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'old_text'"))?;
    let new_text = args.get("new_text").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'new_text'"))?;
    let replace_all = args.get("replace_all").and_then(|v| v.as_bool()).unwrap_or(false);
    let expected_replacements = args.get("expected_replacements").and_then(|v| v.as_u64()).unwrap_or(1) as usize;

    let p = to_abs(path);
    if !p.exists() || !p.is_file() { return Err(anyhow!("file not found")); }
    
    // SAFETY: Validate read-before-edit requirement
    let safety_validator = crate::tool_safety::get_global_safety_validator();
    if let Err(safety_error) = safety_validator.validate_edit_operation(&p) {
        return Err(anyhow!("Safety validation failed: {}", safety_error));
    }

    let mut content = String::new();
    fs::File::open(&p)?.read_to_string(&mut content)?;

    // Check if old_text and new_text are identical
    if old_text == new_text {
        return Err(anyhow!("old_text and new_text are identical - no changes to make"));
    }

    // Count occurrences first for validation
    let occurrence_count = content.matches(old_text).count();
    
    if occurrence_count == 0 {
        return Err(anyhow!("old_text not found in file"));
    }

    // Validate expected replacements
    if !replace_all && occurrence_count != expected_replacements {
        return Err(anyhow!(
            "Expected {} occurrences but found {}. Use replace_all=true for multiple replacements", 
            expected_replacements, occurrence_count
        ));
    }

    let actual_replacements = if replace_all {
        content = content.replace(old_text, new_text);
        occurrence_count
    } else {
        if let Some(pos) = content.find(old_text) {
            content.replace_range(pos..pos + old_text.len(), new_text);
            1
        } else { 0 }
    };

    let mut file = fs::File::create(&p)?;
    file.write_all(content.as_bytes())?;
    
    Ok(json!({
        "ok": true, 
        "replacements": actual_replacements,
        "total_occurrences_found": occurrence_count
    }))
}

fn delete_path(args: Value) -> Result<Value> {
    let path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'path'"))?;
    let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);
    let p = to_abs(path);
    if !p.exists() { return Err(anyhow!("path not found")); }
    
    // SAFETY: Validate path safety for deletion (this is a dangerous operation)
    let safety_validator = crate::tool_safety::get_global_safety_validator();
    if let Err(safety_error) = safety_validator.validate_path(&p) {
        return Err(anyhow!("Safety validation failed: {}", safety_error));
    }

    if p.is_dir() {
        if recursive { fs::remove_dir_all(&p)?; } else { fs::remove_dir(&p)?; }
    } else {
        fs::remove_file(&p)?;
    }
    Ok(json!({"ok": true}))
}

fn search_files(args: Value) -> Result<Value> {
    let pattern = args.get("pattern").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'pattern'"))?;
    let directory = args.get("directory").and_then(|v| v.as_str()).unwrap_or(".");
    let case_sensitive = args.get("case_sensitive").and_then(|v| v.as_bool()).unwrap_or(false);
    let max_results = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
    let pattern_type = args.get("pattern_type").and_then(|v| v.as_str()).unwrap_or("substring");
    let file_pattern = args.get("file_pattern").and_then(|v| v.as_str()).unwrap_or("*");
    let context_lines = args.get("context_lines").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let exclude_dirs = args.get("exclude_dirs").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect::<Vec<_>>())
        .unwrap_or_else(|| vec![".git".to_string(), "node_modules".to_string(), "target".to_string()]);

    let dir = to_abs(directory);
    if !dir.exists() || !dir.is_dir() {
        return Err(anyhow!("directory not found"));
    }

    // Create regex pattern based on pattern type
    let regex = match pattern_type {
        "regex" => {
            let flags = if case_sensitive { "" } else { "(?i)" };
            Regex::new(&format!("{}{}", flags, pattern)).map_err(|_| anyhow!("invalid regex pattern"))?
        },
        "exact" => {
            let flags = if case_sensitive { "" } else { "(?i)" };
            let escaped = regex::escape(pattern);
            Regex::new(&format!("{}\\b{}\\b", flags, escaped)).map_err(|_| anyhow!("invalid pattern"))?
        },
        "fuzzy" => {
            let flags = if case_sensitive { "" } else { "(?i)" };
            let fuzzy_pattern = pattern.chars().map(|c| regex::escape(&c.to_string())).collect::<Vec<_>>().join(".*");
            Regex::new(&format!("{}{}", flags, fuzzy_pattern)).map_err(|_| anyhow!("invalid fuzzy pattern"))?
        },
        _ => { // substring (default)
            let flags = if case_sensitive { "" } else { "(?i)" };
            let escaped = regex::escape(pattern);
            Regex::new(&format!("{}{}", flags, escaped)).map_err(|_| anyhow!("invalid pattern"))?
        }
    };

    let mut matches = Vec::new();
    let mut total_matches = 0;

    for entry in WalkDir::new(&dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path = entry.path();
            
            // Skip excluded directories
            let path_str = path.to_string_lossy();
            if exclude_dirs.iter().any(|excl| path_str.contains(excl)) {
                continue;
            }

            // Check file pattern matching
            if file_pattern != "*" {
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                let pattern_regex = Regex::new(&file_pattern.replace("*", ".*")).unwrap_or_else(|_| Regex::new(".*").unwrap());
                if !pattern_regex.is_match(&file_name) {
                    continue;
                }
            }

            if let Ok(content) = fs::read_to_string(path) {
                let lines: Vec<&str> = content.lines().collect();
                
                for (i, line) in lines.iter().enumerate() {
                    if regex.is_match(line) {
                        let mut context = json!({
                            "file": path.to_string_lossy(),
                            "line": i + 1,
                            "content": line,
                        });

                        // Add context lines if requested
                        if context_lines > 0 {
                            let start_ctx = i.saturating_sub(context_lines);
                            let end_ctx = (i + context_lines + 1).min(lines.len());
                            let context_content: Vec<String> = lines[start_ctx..end_ctx]
                                .iter()
                                .enumerate()
                                .map(|(idx, line)| format!("{}: {}", start_ctx + idx + 1, line))
                                .collect();
                            context["context"] = json!(context_content);
                        }

                        matches.push(context);
                        total_matches += 1;
                        
                        if total_matches >= max_results { 
                            break; 
                        }
                    }
                }
            }
        }
        if total_matches >= max_results { 
            break; 
        }
    }

    Ok(json!({
        "ok": true, 
        "matches": matches,
        "total_matches": total_matches,
        "pattern_type": pattern_type,
        "search_directory": directory
    }))
}

fn execute_command(args: Value) -> Result<Value> {
    let command = args.get("command").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'command'"))?;
    let working_directory = args.get("working_directory").and_then(|v| v.as_str());
    let timeout_secs = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30);
    let command_type = args.get("command_type").and_then(|v| v.as_str()).unwrap_or("bash");

    // Safety check - don't run dangerous commands
    let dangerous_patterns = ["rm -rf /", "sudo rm", "format", "del /s"];
    if dangerous_patterns.iter().any(|&pattern| command.contains(pattern)) {
        return Err(anyhow!("Potentially dangerous command blocked"));
    }

    let start_time = Instant::now();
    
    let mut cmd = match command_type {
        "python" => {
            let mut c = Command::new("python");
            c.arg("-c").arg(command);
            c
        },
        "node" => {
            let mut c = Command::new("node");
            c.arg("-e").arg(command);
            c
        },
        _ => { // bash (default)
            if cfg!(target_os = "windows") {
                let mut c = Command::new("cmd");
                c.arg("/C").arg(command);
                c
            } else {
                let mut c = Command::new("bash");
                c.arg("-c").arg(command);
                c
            }
        }
    };

    if let Some(cwd) = working_directory { 
        cmd.current_dir(cwd); 
    }

    // Set up for timeout handling
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn()?;

    // Simple timeout implementation
    let output = loop {
        if start_time.elapsed() > Duration::from_secs(timeout_secs) {
            child.kill().ok();
            return Err(anyhow!("Command timed out after {} seconds", timeout_secs));
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = child.stdout.take().unwrap();
                let mut stderr = child.stderr.take().unwrap();
                
                let mut stdout_content = String::new();
                let mut stderr_content = String::new();
                
                stdout.read_to_string(&mut stdout_content).ok();
                stderr.read_to_string(&mut stderr_content).ok();

                break std::process::Output {
                    status,
                    stdout: stdout_content.into_bytes(),
                    stderr: stderr_content.into_bytes(),
                };
            },
            Ok(None) => {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            },
            Err(e) => return Err(anyhow!("Error waiting for command: {}", e)),
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status.code().unwrap_or(-1);
    let duration_ms = start_time.elapsed().as_millis();

    Ok(json!({
        "ok": output.status.success(),
        "status": status,
        "stdout": stdout,
        "stderr": stderr,
        "duration_ms": duration_ms,
        "command_type": command_type,
        "working_directory": working_directory
    }))
}

// Task state is now managed by the safe TaskStateManager

fn glob_files(args: Value) -> Result<Value> {
    let pattern = args.get("pattern").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'pattern'"))?;
    let directory = args.get("directory").and_then(|v| v.as_str()).unwrap_or(".");
    let max_results = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(1000) as usize;

    let dir = to_abs(directory);
    if !dir.exists() || !dir.is_dir() {
        return Err(anyhow!("directory not found"));
    }

    let search_pattern = dir.join(pattern).to_string_lossy().to_string();
    let mut matches = Vec::new();

    for entry in glob(&search_pattern).map_err(|_| anyhow!("invalid glob pattern"))? {
        match entry {
            Ok(path) => {
                if matches.len() >= max_results {
                    break;
                }
                let relative_path = path.strip_prefix(&dir).unwrap_or(&path);
                matches.push(json!({
                    "path": relative_path.to_string_lossy(),
                    "absolute_path": path.to_string_lossy(),
                    "is_file": path.is_file(),
                    "is_dir": path.is_dir()
                }));
            },
            Err(_) => continue,
        }
    }

    Ok(json!({
        "ok": true,
        "matches": matches,
        "total_matches": matches.len(),
        "pattern": pattern,
        "directory": directory
    }))
}

fn web_fetch(args: Value) -> Result<Value> {
    let url = args.get("url").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'url'"))?;
    let timeout_secs = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30);

    // Basic URL validation
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(anyhow!("URL must start with http:// or https://"));
    }

    // This is a simplified implementation - in reality you'd use async reqwest
    // For now, we'll use curl as a fallback since reqwest requires async
    let curl_command = format!("curl -L -s --max-time {} '{}'", timeout_secs, url);
    
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .arg("/C")
            .arg(&curl_command)
            .output()
    } else {
        Command::new("bash")
            .arg("-c")
            .arg(&curl_command)
            .output()
    };

    match output {
        Ok(result) => {
            if result.status.success() {
                let content = String::from_utf8_lossy(&result.stdout);
                let content_length = content.len();
                
                // Truncate very large responses
                let truncated_content = if content_length > 50000 {
                    format!("{}... [Content truncated at 50KB]", &content[..50000])
                } else {
                    content.to_string()
                };

                Ok(json!({
                    "ok": true,
                    "content": truncated_content,
                    "content_length": content_length,
                    "url": url,
                    "truncated": content_length > 50000
                }))
            } else {
                let error = String::from_utf8_lossy(&result.stderr);
                Err(anyhow!("Failed to fetch URL: {}", error))
            }
        },
        Err(e) => Err(anyhow!("Failed to execute curl: {}", e))
    }
}

fn create_tasks(args: Value) -> Result<Value> {
    let user_query = args.get("user_query").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'user_query'"))?;
    let tasks = args.get("tasks").and_then(|v| v.as_array()).ok_or_else(|| anyhow!("missing 'tasks' array"))?;

    let task_state = get_global_task_state();
    let task_list = task_state.create_tasks(user_query.to_string(), tasks.clone())?;

    Ok(json!({
        "ok": true,
        "task_list": {
            "user_query": task_list.user_query,
            "tasks": task_list.tasks,
            "created_at": task_list.created_at.to_rfc3339(),
            "updated_at": task_list.updated_at.to_rfc3339()
        },
        "message": format!("Created task list with {} tasks for: {}", task_list.tasks.len(), user_query)
    }))
}

fn update_tasks(args: Value) -> Result<Value> {
    let task_updates = args.get("task_updates").and_then(|v| v.as_array()).ok_or_else(|| anyhow!("missing 'task_updates' array"))?;

    let task_state = get_global_task_state();
    let updates_made = task_state.update_tasks(task_updates)?;
    let current_tasks = task_state.get_tasks()?.ok_or_else(|| anyhow!("No task list exists after update"))?;

    Ok(json!({
        "ok": true,
        "updates_made": updates_made,
        "updated_tasks": current_tasks.tasks,
        "task_statistics": {
            "total_tasks": current_tasks.tasks.len(),
            "updated_at": current_tasks.updated_at.to_rfc3339()
        },
        "message": format!("Updated {} task(s)", updates_made.len())
    }))
}

fn download_file(args: Value) -> Result<Value> {
    let url = args.get("url").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'url'"))?;
    let file_path = args.get("file_path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'file_path'"))?;
    let timeout_secs = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(300); // 5 minutes default

    // Basic URL validation
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(anyhow!("URL must start with http:// or https://"));
    }

    let abs_path = to_abs(file_path);
    
    // Create parent directories if needed
    if let Some(parent) = abs_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Use curl for downloading since we want to keep this simple and not add async
    let curl_command = format!(
        "curl -L -s --max-time {} --output '{}' '{}'", 
        timeout_secs, 
        abs_path.to_string_lossy(), 
        url
    );
    
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .arg("/C")
            .arg(&curl_command)
            .output()
    } else {
        Command::new("bash")
            .arg("-c")
            .arg(&curl_command)
            .output()
    };

    match output {
        Ok(result) => {
            if result.status.success() {
                // Check if file was actually created and get its size
                if abs_path.exists() {
                    let metadata = fs::metadata(&abs_path)?;
                    let file_size = metadata.len();
                    
                    Ok(json!({
                        "ok": true,
                        "file_path": abs_path.to_string_lossy(),
                        "file_size": file_size,
                        "url": url,
                        "message": format!("Successfully downloaded {} bytes to {}", file_size, file_path)
                    }))
                } else {
                    Err(anyhow!("Download completed but file was not created"))
                }
            } else {
                let error = String::from_utf8_lossy(&result.stderr);
                Err(anyhow!("Download failed: {}", error))
            }
        },
        Err(e) => Err(anyhow!("Failed to execute curl: {}", e))
    }
}

fn multiedit_file(args: Value) -> Result<Value> {
    let file_path = args.get("file_path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'file_path'"))?;
    let edits = args.get("edits").and_then(|v| v.as_array()).ok_or_else(|| anyhow!("missing 'edits' array"))?;

    if edits.is_empty() {
        return Err(anyhow!("at least one edit operation is required"));
    }

    let abs_path = to_abs(file_path);
    
    // SAFETY: Validate path safety
    let safety_validator = crate::tool_safety::get_global_safety_validator();
    if let Err(safety_error) = safety_validator.validate_path(&abs_path) {
        return Err(anyhow!("Safety validation failed: {}", safety_error));
    }
    
    // Special case: creating a new file (first edit has empty old_string)
    let is_creating_new_file = edits.get(0)
        .and_then(|e| e.get("old_string"))
        .and_then(|v| v.as_str())
        .map(|s| s.is_empty())
        .unwrap_or(false);

    let mut current_content = if is_creating_new_file {
        if abs_path.exists() {
            return Err(anyhow!("file already exists: {}", file_path));
        }
        // Create parent directories
        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Start with content from first edit
        edits.get(0)
            .and_then(|e| e.get("new_string"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        // Read existing file
        if !abs_path.exists() || !abs_path.is_file() {
            return Err(anyhow!("file not found: {}", file_path));
        }
        
        // SAFETY: Validate read-before-edit for existing file
        if let Err(safety_error) = safety_validator.validate_edit_operation(&abs_path) {
            return Err(anyhow!("Safety validation failed: {}", safety_error));
        }
        
        fs::read_to_string(&abs_path)?
    };

    let original_content = current_content.clone();
    let start_edit_index = if is_creating_new_file { 1 } else { 0 };
    let mut edits_applied = if is_creating_new_file { 1 } else { 0 };

    // Apply each edit sequentially
    for (i, edit) in edits.iter().enumerate().skip(start_edit_index) {
        let edit_obj = edit.as_object().ok_or_else(|| anyhow!("edit {} is not an object", i + 1))?;
        
        let old_string = edit_obj.get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("edit {} missing 'old_string'", i + 1))?;
        
        let new_string = edit_obj.get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("edit {} missing 'new_string'", i + 1))?;
        
        let replace_all = edit_obj.get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Validate edit
        if old_string == new_string {
            return Err(anyhow!("edit {}: old_string and new_string are identical", i + 1));
        }

        if old_string.is_empty() {
            return Err(anyhow!("edit {}: old_string cannot be empty for content replacement", i + 1));
        }

        // Apply the edit
        if replace_all {
            let count = current_content.matches(old_string).count();
            if count == 0 {
                return Err(anyhow!("edit {}: old_string not found in content", i + 1));
            }
            current_content = current_content.replace(old_string, new_string);
        } else {
            let first_occurrence = current_content.find(old_string);
            let last_occurrence = current_content.rfind(old_string);
            
            match (first_occurrence, last_occurrence) {
                (None, _) => {
                    return Err(anyhow!("edit {}: old_string not found in content", i + 1));
                },
                (Some(first), Some(last)) if first != last => {
                    return Err(anyhow!("edit {}: old_string appears multiple times. Use replace_all=true or provide more context", i + 1));
                },
                (Some(pos), _) => {
                    current_content.replace_range(pos..pos + old_string.len(), new_string);
                }
            }
        }
        
        edits_applied += 1;
    }

    // Check if content actually changed
    if !is_creating_new_file && original_content == current_content {
        return Err(anyhow!("no changes made - all edits resulted in identical content"));
    }

    // Write the file
    fs::write(&abs_path, &current_content)?;

    let action = if is_creating_new_file { "created" } else { "modified" };
    
    Ok(json!({
        "ok": true,
        "file_path": abs_path.to_string_lossy(),
        "edits_applied": edits_applied,
        "total_edits": edits.len(),
        "content_length": current_content.len(),
        "message": format!("Successfully {} file with {} edits: {}", action, edits_applied, file_path)
    }))
}

/// Read multiple files efficiently with breadth-first search pattern
fn read_many_files(args: Value) -> Result<Value> {
    let file_paths = args.get("file_paths").and_then(|v| v.as_array()).ok_or_else(|| anyhow!("missing 'file_paths' array"))?;
    let max_file_size = args.get("max_file_size").and_then(|v| v.as_u64()).unwrap_or(1024 * 1024); // 1MB default
    let include_line_numbers = args.get("include_line_numbers").and_then(|v| v.as_bool()).unwrap_or(false);
    
    if file_paths.is_empty() {
        return Err(anyhow!("file_paths array cannot be empty"));
    }
    
    if file_paths.len() > 100 {
        return Err(anyhow!("Cannot read more than 100 files at once"));
    }
    
    let mut results = Vec::new();
    let mut errors = Vec::new();
    let safety_validator = crate::tool_safety::get_global_safety_validator();
    
    for (index, file_path) in file_paths.iter().enumerate() {
        let path_str = file_path.as_str().ok_or_else(|| anyhow!("File path {} is not a string", index))?;
        let abs_path = to_abs(path_str);
        
        match read_single_file_for_batch(&abs_path, max_file_size, include_line_numbers, &safety_validator) {
            Ok(file_result) => results.push(file_result),
            Err(e) => errors.push(json!({
                "path": path_str,
                "error": e.to_string()
            }))
        }
    }
    
    Ok(json!({
        "ok": true,
        "files_read": results.len(),
        "files_failed": errors.len(),
        "results": results,
        "errors": errors,
        "message": format!("Read {} files successfully, {} failed", results.len(), errors.len())
    }))
}

fn read_single_file_for_batch(path: &PathBuf, max_size: u64, include_line_numbers: bool, safety_validator: &crate::tool_safety::ToolSafetyValidator) -> Result<Value> {
    if !path.exists() {
        return Err(anyhow!("File not found: {}", path.display()));
    }
    
    if !path.is_file() {
        return Err(anyhow!("Path is not a file: {}", path.display()));
    }
    
    let metadata = fs::metadata(&path)?;
    if metadata.len() > max_size {
        return Err(anyhow!("File too large: {} bytes (max: {} bytes)", metadata.len(), max_size));
    }
    
    let content = fs::read_to_string(&path)?;
    
    // Record file read for safety validation
    safety_validator.record_file_read(&path);
    
    let line_count = content.lines().count();
    let processed_content = if include_line_numbers {
        content.lines()
            .enumerate()
            .map(|(i, line)| format!("{:4}: {}", i + 1, line))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        content
    };
    
    Ok(json!({
        "path": path.to_string_lossy(),
        "size": metadata.len(),
        "lines": line_count,
        "content": processed_content,
        "modified": metadata.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
    }))
}

/// Compare two files and show differences
fn diff_files(args: Value) -> Result<Value> {
    let file1_path = args.get("file1").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'file1' path"))?;
    let file2_path = args.get("file2").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'file2' path"))?;
    let diff_type = args.get("diff_type").and_then(|v| v.as_str()).unwrap_or("unified"); // unified, side-by-side
    let context_lines = args.get("context_lines").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
    
    let path1 = to_abs(file1_path);
    let path2 = to_abs(file2_path);
    
    if !path1.exists() {
        return Err(anyhow!("File 1 not found: {}", path1.display()));
    }
    
    if !path2.exists() {
        return Err(anyhow!("File 2 not found: {}", path2.display()));
    }
    
    let content1 = fs::read_to_string(&path1)?;
    let content2 = fs::read_to_string(&path2)?;
    
    // Record reads for safety validation
    let safety_validator = crate::tool_safety::get_global_safety_validator();
    safety_validator.record_file_read(&path1);
    safety_validator.record_file_read(&path2);
    
    let lines1: Vec<&str> = content1.lines().collect();
    let lines2: Vec<&str> = content2.lines().collect();
    
    // Simple diff implementation using longest common subsequence approach
    let diff_result = compute_simple_diff(&lines1, &lines2, context_lines);
    
    Ok(json!({
        "ok": true,
        "file1": file1_path,
        "file2": file2_path,
        "file1_lines": lines1.len(),
        "file2_lines": lines2.len(),
        "diff_type": diff_type,
        "changes": diff_result.changes,
        "additions": diff_result.additions,
        "deletions": diff_result.deletions,
        "diff_output": diff_result.diff_text,
        "identical": diff_result.identical
    }))
}

#[derive(Debug)]
struct DiffResult {
    changes: usize,
    additions: usize,
    deletions: usize,
    diff_text: String,
    identical: bool,
}

fn compute_simple_diff(lines1: &[&str], lines2: &[&str], _context_lines: usize) -> DiffResult {
    if lines1 == lines2 {
        return DiffResult {
            changes: 0,
            additions: 0,
            deletions: 0,
            diff_text: "Files are identical".to_string(),
            identical: true,
        };
    }
    
    let mut diff_output = Vec::new();
    let mut additions = 0;
    let mut deletions = 0;
    
    // Simple line-by-line comparison (not optimal but functional)
    let max_lines = lines1.len().max(lines2.len());
    let mut i = 0;
    
    while i < max_lines {
        let line1 = lines1.get(i);
        let line2 = lines2.get(i);
        
        match (line1, line2) {
            (Some(l1), Some(l2)) => {
                if l1 != l2 {
                    diff_output.push(format!("- {}: {}", i + 1, l1));
                    diff_output.push(format!("+ {}: {}", i + 1, l2));
                    deletions += 1;
                    additions += 1;
                }
            }
            (Some(l1), None) => {
                diff_output.push(format!("- {}: {}", i + 1, l1));
                deletions += 1;
            }
            (None, Some(l2)) => {
                diff_output.push(format!("+ {}: {}", i + 1, l2));
                additions += 1;
            }
            (None, None) => break,
        }
        i += 1;
    }
    
    DiffResult {
        changes: additions + deletions,
        additions,
        deletions,
        diff_text: diff_output.join("\n"),
        identical: false,
    }
}

/// Batch search across multiple files with advanced filtering
fn batch_file_search(args: Value) -> Result<Value> {
    let pattern = args.get("pattern").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'pattern'"))?;
    let default_dirs = vec![json!(".")];
    let directories = args.get("directories").and_then(|v| v.as_array()).unwrap_or(&default_dirs);
    let file_extensions = args.get("file_extensions").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["rs".to_string(), "toml".to_string(), "md".to_string(), "json".to_string()]);
    let max_results = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(500) as usize;
    let case_sensitive = args.get("case_sensitive").and_then(|v| v.as_bool()).unwrap_or(false);
    let include_content = args.get("include_content").and_then(|v| v.as_bool()).unwrap_or(false);
    let max_file_size = args.get("max_file_size").and_then(|v| v.as_u64()).unwrap_or(1024 * 1024); // 1MB
    
    // Build regex pattern
    let flags = if case_sensitive { "" } else { "(?i)" };
    let regex_pattern = format!("{}{}", flags, regex::escape(pattern));
    let regex = Regex::new(&regex_pattern).map_err(|_| anyhow!("Invalid regex pattern"))?;
    
    let mut all_matches = Vec::new();
    let mut files_scanned = 0;
    let mut total_matches = 0;
    
    for dir_value in directories {
        let dir_str = dir_value.as_str().unwrap_or(".");
        let dir_path = to_abs(dir_str);
        
        if !dir_path.exists() || !dir_path.is_dir() {
            continue;
        }
        
        for entry in WalkDir::new(&dir_path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                files_scanned += 1;
                
                let path = entry.path();
                
                // Check file extension
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if !file_extensions.contains(&ext.to_string()) {
                        continue;
                    }
                } else if !file_extensions.is_empty() {
                    continue;
                }
                
                // Check file size
                if let Ok(metadata) = path.metadata() {
                    if metadata.len() > max_file_size {
                        continue;
                    }
                }
                
                if let Ok(content) = fs::read_to_string(path) {
                    let mut file_matches = Vec::new();
                    
                    for (line_num, line) in content.lines().enumerate() {
                        if regex.is_match(line) {
                            file_matches.push(json!({
                                "line": line_num + 1,
                                "content": if include_content { line } else { "" },
                                "match_found": true
                            }));
                            
                            total_matches += 1;
                            if total_matches >= max_results {
                                break;
                            }
                        }
                    }
                    
                    if !file_matches.is_empty() {
                        let matches_count = file_matches.len();
                        let details = if include_content { 
                            Value::Array(file_matches) 
                        } else { 
                            json!([]) 
                        };
                        
                        all_matches.push(json!({
                            "file": path.to_string_lossy(),
                            "matches": matches_count,
                            "details": details
                        }));
                    }
                }
                
                if total_matches >= max_results {
                    break;
                }
            }
        }
        
        if total_matches >= max_results {
            break;
        }
    }
    
    Ok(json!({
        "ok": true,
        "pattern": pattern,
        "files_scanned": files_scanned,
        "files_with_matches": all_matches.len(),
        "total_matches": total_matches,
        "results": all_matches,
        "max_results_reached": total_matches >= max_results,
        "message": format!("Found {} matches across {} files (scanned {} files)", 
                          total_matches, all_matches.len(), files_scanned)
    }))
}
