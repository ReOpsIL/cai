use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use directories::ProjectDirs;

use crate::chat::{Prompt, get_memory};

lazy_static! {
    static ref SESSION_MANAGER: Mutex<SessionManager> = Mutex::new(SessionManager::new());
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    pub created: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub memory: HashMap<String, Prompt>,
    pub config_overrides: Option<SessionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
}

#[derive(Debug)]
pub struct SessionManager {
    pub current_session: Option<String>,
    pub sessions_dir: PathBuf,
    pub auto_save: bool,
}

impl SessionManager {
    pub fn new() -> Self {
        let sessions_dir = if let Some(proj_dirs) = ProjectDirs::from("", "", "cai") {
            let mut dir = proj_dirs.data_dir().to_path_buf();
            dir.push("sessions");
            dir
        } else {
            PathBuf::from(".cai/sessions")
        };

        // Ensure sessions directory exists
        if !sessions_dir.exists() {
            if let Err(e) = fs::create_dir_all(&sessions_dir) {
                eprintln!("Warning: Could not create sessions directory: {}", e);
            }
        }

        SessionManager {
            current_session: None,
            sessions_dir,
            auto_save: true,
        }
    }

    pub fn create_session(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if name.is_empty() || name.contains('/') || name.contains('\\') {
            return Err("Invalid session name".into());
        }

        let session = Session {
            name: name.to_string(),
            created: Utc::now(),
            last_accessed: Utc::now(),
            memory: HashMap::new(),
            config_overrides: None,
        };

        self.save_session(&session)?;
        Ok(())
    }

    pub fn switch_to_session(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Save current session if auto_save is enabled
        if self.auto_save {
            if let Some(current_name) = &self.current_session {
                self.save_current_memory_to_session(current_name)?;
            }
        }

        // Load the new session
        let mut session = self.load_session(name)?;
        session.last_accessed = Utc::now();
        
        // Clear current memory and load session memory
        {
            let mut memory = get_memory().lock().unwrap();
            memory.clear();
            for (key, value) in &session.memory {
                memory.insert(key.clone(), value.clone());
            }
        }

        // Save the updated last_accessed time
        self.save_session(&session)?;
        self.current_session = Some(name.to_string());

        Ok(())
    }

    pub fn save_current_memory_to_session(&self, session_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut session = self.load_session(session_name)?;
        
        // Copy current memory to session
        {
            let memory = get_memory().lock().unwrap();
            session.memory.clear();
            for (key, value) in memory.iter() {
                session.memory.insert(key.clone(), value.clone());
            }
        }
        
        session.last_accessed = Utc::now();
        self.save_session(&session)?;
        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut sessions = Vec::new();
        
        if !self.sessions_dir.exists() {
            return Ok(sessions);
        }

        for entry in fs::read_dir(&self.sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    if let Some(name) = stem.to_str() {
                        sessions.push(name.to_string());
                    }
                }
            }
        }

        sessions.sort();
        Ok(sessions)
    }

    pub fn delete_session(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let session_path = self.get_session_path(name);
        if session_path.exists() {
            fs::remove_file(session_path)?;
            Ok(())
        } else {
            Err(format!("Session '{}' not found", name).into())
        }
    }

    pub fn export_session(&self, name: &str, export_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let session = self.load_session(name)?;
        
        let mut content = String::new();
        content.push_str(&format!("# Session: {}\n", session.name));
        content.push_str(&format!("Created: {}\n", session.created.format("%Y-%m-%d %H:%M:%S")));
        content.push_str(&format!("Last Accessed: {}\n\n", session.last_accessed.format("%Y-%m-%d %H:%M:%S")));

        // Sort prompts by date
        let mut prompts: Vec<&Prompt> = session.memory.values().collect();
        prompts.sort_by(|a, b| a.date.cmp(&b.date));

        for prompt in prompts {
            content.push_str(&format!("## {} ({})\n", prompt.id, prompt.date.format("%Y-%m-%d %H:%M:%S")));
            content.push_str(&format!("Type: {:?}\n\n", prompt.ptype));
            content.push_str(&prompt.value);
            content.push_str("\n\n---\n\n");
        }

        // Ensure parent directory exists
        if let Some(parent) = Path::new(export_path).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::write(export_path, content)?;
        Ok(())
    }

    pub fn get_current_session(&self) -> Option<&String> {
        self.current_session.as_ref()
    }

    pub fn get_session_info(&self, name: &str) -> Result<Session, Box<dyn std::error::Error>> {
        self.load_session(name)
    }

    fn save_session(&self, session: &Session) -> Result<(), Box<dyn std::error::Error>> {
        let session_path = self.get_session_path(&session.name);
        let json = serde_json::to_string_pretty(session)?;
        fs::write(session_path, json)?;
        Ok(())
    }

    fn load_session(&self, name: &str) -> Result<Session, Box<dyn std::error::Error>> {
        let session_path = self.get_session_path(name);
        if !session_path.exists() {
            return Err(format!("Session '{}' not found", name).into());
        }
        
        let json = fs::read_to_string(session_path)?;
        let session: Session = serde_json::from_str(&json)?;
        Ok(session)
    }

    fn get_session_path(&self, name: &str) -> PathBuf {
        let mut path = self.sessions_dir.clone();
        path.push(format!("{}.json", name));
        path
    }
}

// Public API functions
pub fn get_session_manager() -> &'static Mutex<SessionManager> {
    &SESSION_MANAGER
}

pub fn create_session(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut manager = get_session_manager().lock().unwrap();
    manager.create_session(name)?;
    Ok(format!("Session '{}' created successfully", name))
}

pub fn switch_session(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut manager = get_session_manager().lock().unwrap();
    manager.switch_to_session(name)?;
    Ok(format!("Switched to session '{}'", name))
}

pub fn list_sessions() -> Result<String, Box<dyn std::error::Error>> {
    let manager = get_session_manager().lock().unwrap();
    let sessions = manager.list_sessions()?;
    
    if sessions.is_empty() {
        return Ok("No sessions found".to_string());
    }

    let mut result = String::from("Available sessions:\n");
    for session_name in &sessions {
        let current_marker = if manager.get_current_session() == Some(session_name) {
            " (current)"
        } else {
            ""
        };
        
        match manager.get_session_info(session_name) {
            Ok(session) => {
                result.push_str(&format!(
                    "- {} {}\n  Created: {}, Last accessed: {}\n",
                    session_name,
                    current_marker,
                    session.created.format("%Y-%m-%d %H:%M"),
                    session.last_accessed.format("%Y-%m-%d %H:%M")
                ));
            }
            Err(_) => {
                result.push_str(&format!("- {} {} (error loading details)\n", session_name, current_marker));
            }
        }
    }
    
    Ok(result)
}

pub fn delete_session(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let manager = get_session_manager().lock().unwrap();
    manager.delete_session(name)?;
    Ok(format!("Session '{}' deleted successfully", name))
}

pub fn export_session(name: &str, export_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let manager = get_session_manager().lock().unwrap();
    manager.export_session(name, export_path)?;
    Ok(format!("Session '{}' exported to '{}'", name, export_path))
}

pub fn get_current_session_info() -> Result<String, Box<dyn std::error::Error>> {
    let manager = get_session_manager().lock().unwrap();
    
    match manager.get_current_session() {
        Some(name) => {
            match manager.get_session_info(name) {
                Ok(session) => {
                    let memory_count = session.memory.len();
                    Ok(format!(
                        "Current session: '{}'\nCreated: {}\nLast accessed: {}\nMemory items: {}",
                        name,
                        session.created.format("%Y-%m-%d %H:%M:%S"),
                        session.last_accessed.format("%Y-%m-%d %H:%M:%S"),
                        memory_count
                    ))
                }
                Err(e) => Ok(format!("Current session: '{}' (error: {})", name, e))
            }
        }
        None => Ok("No active session".to_string())
    }
}

pub fn save_current_session() -> Result<String, Box<dyn std::error::Error>> {
    let manager = get_session_manager().lock().unwrap();
    
    match manager.get_current_session() {
        Some(name) => {
            manager.save_current_memory_to_session(name)?;
            Ok(format!("Current session '{}' saved", name))
        }
        None => Ok("No active session to save".to_string())
    }
}