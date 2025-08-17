use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use uuid::Uuid;

pub type SessionId = String;
pub type PermissionId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionManager {
    /// Session-based permissions (from crush pattern)
    session_permissions: HashMap<SessionId, Vec<Permission>>,
    /// Tool allowlists (from Claude Code pattern)
    allowed_tools: HashSet<String>,
    /// Path-based access control (from crush pattern)
    trusted_paths: Vec<PathBuf>,
    /// Persistent permission storage
    permission_storage: PermissionStorage,
    /// Auto-approval settings for non-interactive operations
    auto_approval_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: PermissionId,
    pub tool_name: String,
    pub action: PermissionAction,
    pub path: Option<PathBuf>,
    pub expires_at: Option<u64>, // Unix timestamp
    pub scope: PermissionScope,
    pub granted_at: u64,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionAction {
    Read,
    Write,
    Execute,
    Delete,
    Create,
    ModifyPermissions,
    NetworkAccess,
    EnvironmentAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionScope {
    Session,    // Valid for current session only
    Permanent,  // Persists across sessions
    Project,    // Valid for current project directory
    OneTime,    // Single use permission
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: PermissionId,
    pub session_id: SessionId,
    pub tool_call_id: Option<String>,
    pub tool_name: String,
    pub action: PermissionAction,
    pub path: Option<PathBuf>,
    pub description: String,
    pub justification: String,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,      // Read operations, safe tools
    Medium,   // Write operations, file creation
    High,     // Delete operations, system access
    Critical, // Permission changes, network access
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionResponse {
    Granted(Permission),
    Denied { reason: String },
    RequiresApproval { prompt: String, options: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionStorage {
    storage_path: PathBuf,
    permissions: HashMap<SessionId, Vec<Permission>>,
    global_allowlist: HashSet<String>,
}

impl PermissionManager {
    pub async fn new(storage_path: PathBuf) -> Result<Self> {
        let permission_storage = PermissionStorage::load(&storage_path).await?;
        
        Ok(Self {
            session_permissions: HashMap::new(),
            allowed_tools: permission_storage.global_allowlist.clone(),
            trusted_paths: vec![
                dirs::home_dir().unwrap_or_default().join("Projects"),
                std::env::current_dir()?,
            ],
            permission_storage,
            auto_approval_enabled: false,
        })
    }

    /// Request permission for a specific operation
    pub async fn request_permission(
        &mut self,
        request: PermissionRequest,
    ) -> Result<PermissionResponse> {
        // Check if tool is globally allowed
        if self.allowed_tools.contains(&request.tool_name) {
            let permission = self.grant_permission(&request)?;
            return Ok(PermissionResponse::Granted(permission));
        }

        // Check existing session permissions
        if let Some(existing) = self.check_existing_permission(&request) {
            return Ok(PermissionResponse::Granted(existing));
        }

        // Risk-based approval logic
        match request.risk_level {
            RiskLevel::Low => {
                if self.is_safe_operation(&request) {
                    let permission = self.grant_permission(&request)?;
                    Ok(PermissionResponse::Granted(permission))
                } else {
                    self.require_approval(&request)
                }
            }
            RiskLevel::Medium => {
                if self.auto_approval_enabled && self.is_trusted_path(&request.path) {
                    let permission = self.grant_permission(&request)?;
                    Ok(PermissionResponse::Granted(permission))
                } else {
                    self.require_approval(&request)
                }
            }
            RiskLevel::High | RiskLevel::Critical => {
                self.require_approval(&request)
            }
        }
    }

    /// Check if operation is permitted
    pub fn check_permission(
        &self,
        session_id: &SessionId,
        tool_name: &str,
        action: &PermissionAction,
        path: Option<&Path>,
    ) -> Result<bool> {
        // Check global allowlist
        if self.allowed_tools.contains(tool_name) {
            return Ok(true);
        }

        // Check session permissions
        if let Some(permissions) = self.session_permissions.get(session_id) {
            for permission in permissions {
                if self.permission_matches(permission, tool_name, action, path)? {
                    // Check if permission is still valid
                    if !self.is_permission_expired(permission)? {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Grant permission and store it
    fn grant_permission(&mut self, request: &PermissionRequest) -> Result<Permission> {
        let permission = Permission {
            id: Uuid::new_v4().to_string(),
            tool_name: request.tool_name.clone(),
            action: request.action.clone(),
            path: request.path.clone(),
            expires_at: self.calculate_expiration(&request.scope),
            scope: request.scope.clone(),
            granted_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs(),
            description: request.description.clone(),
        };

        // Store in session
        self.session_permissions
            .entry(request.session_id.clone())
            .or_insert_with(Vec::new)
            .push(permission.clone());

        // Store permanently if needed
        if matches!(permission.scope, PermissionScope::Permanent | PermissionScope::Project) {
            self.permission_storage.store_permission(&permission)?;
        }

        Ok(permission)
    }

    /// Check if there's an existing valid permission
    fn check_existing_permission(&self, request: &PermissionRequest) -> Option<Permission> {
        let permissions = self.session_permissions.get(&request.session_id)?;
        
        permissions.iter().find(|p| {
            p.tool_name == request.tool_name
                && p.action == request.action
                && p.path == request.path
                && !self.is_permission_expired(p).unwrap_or(true)
        }).cloned()
    }

    /// Require user approval for the operation
    fn require_approval(&self, request: &PermissionRequest) -> Result<PermissionResponse> {
        let prompt = format!(
            "Permission required for {}:\n\n\
            Tool: {}\n\
            Action: {:?}\n\
            Path: {}\n\
            Risk Level: {:?}\n\
            Description: {}\n\
            Justification: {}\n\n\
            Grant permission?",
            request.description,
            request.tool_name,
            request.action,
            request.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "N/A".to_string()),
            request.risk_level,
            request.description,
            request.justification
        );

        let options = vec![
            "Grant once".to_string(),
            "Grant for session".to_string(),
            "Grant permanently".to_string(),
            "Deny".to_string(),
        ];

        Ok(PermissionResponse::RequiresApproval { prompt, options })
    }

    /// Check if operation is considered safe
    fn is_safe_operation(&self, request: &PermissionRequest) -> bool {
        matches!(request.action, PermissionAction::Read)
            && request.path.as_ref().map_or(true, |p| self.is_trusted_path(&Some(p.clone())))
            && matches!(request.risk_level, RiskLevel::Low)
    }

    /// Check if path is in trusted locations
    fn is_trusted_path(&self, path: &Option<PathBuf>) -> bool {
        let Some(path) = path else { return false };
        
        self.trusted_paths.iter().any(|trusted| {
            path.starts_with(trusted)
        })
    }

    /// Check if permission matches the requested operation
    fn permission_matches(
        &self,
        permission: &Permission,
        tool_name: &str,
        action: &PermissionAction,
        path: Option<&Path>,
    ) -> Result<bool> {
        if permission.tool_name != tool_name || permission.action != *action {
            return Ok(false);
        }

        // Check path matching
        match (&permission.path, path) {
            (None, _) => Ok(true), // Global permission
            (Some(perm_path), Some(req_path)) => {
                // Path must match exactly or be a parent directory
                Ok(req_path.starts_with(perm_path))
            }
            (Some(_), None) => Ok(false), // Specific path required but not provided
        }
    }

    /// Check if permission has expired
    fn is_permission_expired(&self, permission: &Permission) -> Result<bool> {
        let Some(expires_at) = permission.expires_at else {
            return Ok(false); // No expiration
        };

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        Ok(current_time > expires_at)
    }

    /// Calculate expiration time based on scope
    fn calculate_expiration(&self, scope: &PermissionScope) -> Option<u64> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs();

        match scope {
            PermissionScope::Session => Some(current_time + 24 * 60 * 60), // 24 hours
            PermissionScope::OneTime => Some(current_time + 60), // 1 minute
            PermissionScope::Project => Some(current_time + 7 * 24 * 60 * 60), // 1 week
            PermissionScope::Permanent => None, // No expiration
        }
    }

    /// Add tool to global allowlist
    pub async fn add_to_allowlist(&mut self, tool_name: String) -> Result<()> {
        self.allowed_tools.insert(tool_name.clone());
        self.permission_storage.global_allowlist.insert(tool_name);
        self.permission_storage.save().await?;
        Ok(())
    }

    /// Remove tool from global allowlist
    pub async fn remove_from_allowlist(&mut self, tool_name: &str) -> Result<()> {
        self.allowed_tools.remove(tool_name);
        self.permission_storage.global_allowlist.remove(tool_name);
        self.permission_storage.save().await?;
        Ok(())
    }

    /// Revoke specific permission
    pub async fn revoke_permission(&mut self, session_id: &SessionId, permission_id: &PermissionId) -> Result<()> {
        if let Some(permissions) = self.session_permissions.get_mut(session_id) {
            permissions.retain(|p| p.id != *permission_id);
        }
        
        self.permission_storage.revoke_permission(permission_id).await?;
        Ok(())
    }

    /// Clear all session permissions
    pub fn clear_session_permissions(&mut self, session_id: &SessionId) {
        self.session_permissions.remove(session_id);
    }

    /// Enable/disable auto-approval for trusted operations
    pub fn set_auto_approval(&mut self, enabled: bool) {
        self.auto_approval_enabled = enabled;
    }

    /// Get all permissions for a session
    pub fn get_session_permissions(&self, session_id: &SessionId) -> Vec<Permission> {
        self.session_permissions
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }
}

impl PermissionStorage {
    async fn load(storage_path: &Path) -> Result<Self> {
        if storage_path.exists() {
            let content = fs::read_to_string(storage_path).await?;
            let storage: PermissionStorage = serde_json::from_str(&content)?;
            Ok(storage)
        } else {
            Ok(Self {
                storage_path: storage_path.to_path_buf(),
                permissions: HashMap::new(),
                global_allowlist: HashSet::new(),
            })
        }
    }

    async fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&self.storage_path, content).await?;
        Ok(())
    }

    fn store_permission(&mut self, permission: &Permission) -> Result<()> {
        // For permanent permissions, we need a way to associate them with sessions
        // For now, store them under a special "permanent" session
        let session_key = match permission.scope {
            PermissionScope::Permanent => "permanent".to_string(),
            PermissionScope::Project => format!("project:{}", 
                permission.path.as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ),
            _ => return Ok(()), // Don't store session-only permissions
        };

        self.permissions
            .entry(session_key)
            .or_insert_with(Vec::new)
            .push(permission.clone());

        Ok(())
    }

    async fn revoke_permission(&mut self, permission_id: &PermissionId) -> Result<()> {
        for permissions in self.permissions.values_mut() {
            permissions.retain(|p| p.id != *permission_id);
        }
        self.save().await?;
        Ok(())
    }

    /// Get statistics about permission usage
    pub async fn get_statistics(&self) -> PermissionStatistics {
        PermissionStatistics {
            total_requests: self.session_permissions.len() as u64,
            granted_permissions: self.session_permissions.values()
                .map(|perms| perms.len())
                .sum::<usize>() as u64,
            denied_permissions: 0, // TODO: Track denials
            active_sessions: self.session_permissions.len() as u64,
            trusted_paths: self.trusted_paths.len() as u64,
            allowed_tools: self.allowed_tools.len() as u64,
        }
    }
}

/// Statistics about permission usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionStatistics {
    pub total_requests: u64,
    pub granted_permissions: u64,
    pub denied_permissions: u64,
    pub active_sessions: u64,
    pub trusted_paths: u64,
    pub allowed_tools: u64,
}

impl std::fmt::Display for PermissionAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionAction::Read => write!(f, "read"),
            PermissionAction::Write => write!(f, "write"),
            PermissionAction::Execute => write!(f, "execute"),
            PermissionAction::Delete => write!(f, "delete"),
            PermissionAction::Create => write!(f, "create"),
            PermissionAction::ModifyPermissions => write!(f, "modify permissions"),
            PermissionAction::NetworkAccess => write!(f, "network access"),
            PermissionAction::EnvironmentAccess => write!(f, "environment access"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_permission_request_and_grant() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("permissions.json");
        
        let mut manager = PermissionManager::new(storage_path).await.unwrap();
        
        let request = PermissionRequest {
            id: "test-request".to_string(),
            session_id: "test-session".to_string(),
            tool_call_id: None,
            tool_name: "test-tool".to_string(),
            action: PermissionAction::Read,
            path: None,
            description: "Test operation".to_string(),
            justification: "Testing permission system".to_string(),
            risk_level: RiskLevel::Low,
        };

        let response = manager.request_permission(request).await.unwrap();
        
        // Low risk read operation should be granted automatically
        assert!(matches!(response, PermissionResponse::Granted(_)));
    }

    #[tokio::test]
    async fn test_permission_checking() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("permissions.json");
        
        let mut manager = PermissionManager::new(storage_path).await.unwrap();
        
        // Add tool to allowlist
        manager.add_to_allowlist("allowed-tool".to_string()).await.unwrap();
        
        // Check permission for allowed tool
        let has_permission = manager.check_permission(
            &"test-session".to_string(),
            "allowed-tool",
            &PermissionAction::Read,
            None,
        ).unwrap();
        
        assert!(has_permission);
        
        // Check permission for non-allowed tool
        let has_permission = manager.check_permission(
            &"test-session".to_string(),
            "unknown-tool",
            &PermissionAction::Read,
            None,
        ).unwrap();
        
        assert!(!has_permission);
    }
}