use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use sha2::{Digest, Sha256};

use crate::logger::{log_debug, log_error, log_info, log_warn};

/// File Operation Safety System inspired by Crush
/// Prevents data loss through comprehensive safety checks and backup management
#[derive(Debug)]
pub struct FileOperationSafety {
    /// Track when files were last read by CAI
    file_read_times: Mutex<HashMap<PathBuf, SystemTime>>,
    /// Track file content checksums to detect changes
    file_checksums: Mutex<HashMap<PathBuf, String>>,
    /// Backup manager for file versioning
    backup_manager: BackupManager,
    /// Safety configuration
    config: SafetyConfig,
}

#[derive(Debug, Clone)]
pub struct SafetyConfig {
    /// Enable backup creation before modifications
    pub enable_backups: bool,
    /// Maximum number of backups to keep per file
    pub max_backups: usize,
    /// Check modification times before writing
    pub check_modification_times: bool,
    /// Verify checksums before writing
    pub verify_checksums: bool,
    /// Require explicit consent for destructive operations
    pub require_destructive_consent: bool,
    /// Backup directory path
    pub backup_directory: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    pub path: PathBuf,
    pub version: u32,
    pub checksum: String,
    pub created_at: SystemTime,
    pub backup_path: Option<PathBuf>,
    pub operation_type: OperationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Create,
    Modify,
    Delete,
    Move { from: PathBuf, to: PathBuf },
}

#[derive(Debug, Clone)]
pub struct BackupManager {
    backup_dir: PathBuf,
    max_backups: usize,
}

#[derive(Debug, Clone)]
pub enum SafetyError {
    FileModifiedExternally {
        path: PathBuf,
        last_read: SystemTime,
        last_modified: SystemTime,
    },
    ChecksumMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    BackupFailed {
        path: PathBuf,
        reason: String,
    },
    DestructiveOperationBlocked {
        path: PathBuf,
        operation: String,
    },
    PermissionDenied {
        path: PathBuf,
        operation: String,
    },
}

impl std::fmt::Display for SafetyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SafetyError::FileModifiedExternally { path, last_read, last_modified } => {
                write!(f, "File {} has been modified externally. Last read: {:?}, Last modified: {:?}", 
                       path.display(), last_read, last_modified)
            }
            SafetyError::ChecksumMismatch { path, expected, actual } => {
                write!(f, "Checksum mismatch for file {}. Expected: {}, Actual: {}", 
                       path.display(), expected, actual)
            }
            SafetyError::BackupFailed { path, reason } => {
                write!(f, "Failed to backup file {}: {}", path.display(), reason)
            }
            SafetyError::DestructiveOperationBlocked { path, operation } => {
                write!(f, "Destructive operation '{}' blocked for file {}", operation, path.display())
            }
            SafetyError::PermissionDenied { path, operation } => {
                write!(f, "Permission denied for operation '{}' on file {}", operation, path.display())
            }
        }
    }
}

impl std::error::Error for SafetyError {}

impl FileOperationSafety {
    pub fn new(config: SafetyConfig) -> Result<Self> {
        let backup_manager = BackupManager::new(config.backup_directory.clone(), config.max_backups);
        
        Ok(Self {
            file_read_times: Mutex::new(HashMap::new()),
            file_checksums: Mutex::new(HashMap::new()),
            backup_manager,
            config,
        })
    }

    pub fn with_defaults(backup_dir: PathBuf) -> Result<Self> {
        let config = SafetyConfig {
            enable_backups: true,
            max_backups: 10,
            check_modification_times: true,
            verify_checksums: true,
            require_destructive_consent: true,
            backup_directory: backup_dir,
        };
        Self::new(config)
    }

    /// Record that a file has been read by CAI
    pub async fn record_file_read(&self, path: &Path) -> Result<()> {
        let now = SystemTime::now();
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        
        // Record read time
        let mut read_times = self.file_read_times.lock().await;
        read_times.insert(canonical_path.clone(), now);
        
        // Calculate and store checksum if file exists
        if canonical_path.exists() {
            let content = fs::read(&canonical_path)?;
            let checksum = self.calculate_checksum(&content);
            
            let mut checksums = self.file_checksums.lock().await;
            checksums.insert(canonical_path.clone(), checksum);
        }
        
        log_debug!("file_safety", "Recorded read for file: {}", canonical_path.display());
        Ok(())
    }

    /// Validate a file write operation for safety
    pub async fn validate_write(&self, path: &Path, new_content: &str) -> Result<(), SafetyError> {
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        
        log_debug!("file_safety", "Validating write operation for: {}", canonical_path.display());
        
        // Check if file exists
        let file_exists = canonical_path.exists();
        
        if file_exists {
            // Check modification time if enabled
            if self.config.check_modification_times {
                self.check_modification_time(&canonical_path).await?;
            }
            
            // Verify checksum if enabled
            if self.config.verify_checksums {
                self.verify_checksum(&canonical_path).await?;
            }
        }
        
        // Check for destructive operations
        if self.config.require_destructive_consent && file_exists {
            // For now, we'll log this - in a full implementation, this would
            // trigger a user confirmation dialog
            log_warn!("file_safety", "Modifying existing file: {}", canonical_path.display());
        }
        
        log_info!("file_safety", "File write validation passed for: {}", canonical_path.display());
        Ok(())
    }

    /// Safely write content to a file with backup
    pub async fn safe_write(&self, path: &Path, content: &str) -> Result<()> {
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        
        // Validate the write operation
        self.validate_write(&canonical_path, content).await
            .map_err(|e| anyhow!("Write validation failed: {}", e))?;
        
        // Create backup if file exists and backups are enabled
        if self.config.enable_backups && canonical_path.exists() {
            self.backup_manager.create_backup(&canonical_path).await
                .map_err(|e| anyhow!("Failed to create backup: {}", e))?;
        }
        
        // Create parent directories if needed
        if let Some(parent) = canonical_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Write the new content
        fs::write(&canonical_path, content)?;
        
        // Update our tracking information
        self.record_file_read(&canonical_path).await?;
        
        log_info!("file_safety", "Successfully wrote file: {}", canonical_path.display());
        Ok(())
    }

    /// Safely delete a file with backup
    pub async fn safe_delete(&self, path: &Path) -> Result<()> {
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        
        if !canonical_path.exists() {
            return Ok(()); // Already deleted
        }
        
        // Check for destructive operation consent
        if self.config.require_destructive_consent {
            log_warn!("file_safety", "Destructive operation: deleting file {}", canonical_path.display());
            // In a full implementation, this would trigger user confirmation
        }
        
        // Create backup before deletion
        if self.config.enable_backups {
            self.backup_manager.create_backup(&canonical_path).await
                .map_err(|e| anyhow!("Failed to backup before deletion: {}", e))?;
        }
        
        // Remove from tracking
        let mut read_times = self.file_read_times.lock().await;
        let mut checksums = self.file_checksums.lock().await;
        read_times.remove(&canonical_path);
        checksums.remove(&canonical_path);
        
        // Delete the file
        fs::remove_file(&canonical_path)?;
        
        log_info!("file_safety", "Successfully deleted file: {}", canonical_path.display());
        Ok(())
    }

    /// Check if file has been modified since last read
    async fn check_modification_time(&self, path: &Path) -> Result<(), SafetyError> {
        let read_times = self.file_read_times.lock().await;
        
        if let Some(last_read) = read_times.get(path) {
            let metadata = fs::metadata(path)
                .map_err(|_| SafetyError::PermissionDenied {
                    path: path.to_path_buf(),
                    operation: "read metadata".to_string(),
                })?;
            
            let last_modified = metadata.modified()
                .map_err(|_| SafetyError::PermissionDenied {
                    path: path.to_path_buf(),
                    operation: "read modification time".to_string(),
                })?;
            
            if last_modified > *last_read {
                return Err(SafetyError::FileModifiedExternally {
                    path: path.to_path_buf(),
                    last_read: *last_read,
                    last_modified,
                });
            }
        }
        
        Ok(())
    }

    /// Verify file checksum matches our recorded value
    async fn verify_checksum(&self, path: &Path) -> Result<(), SafetyError> {
        let checksums = self.file_checksums.lock().await;
        
        if let Some(expected_checksum) = checksums.get(path) {
            let content = fs::read(path)
                .map_err(|_| SafetyError::PermissionDenied {
                    path: path.to_path_buf(),
                    operation: "read for checksum".to_string(),
                })?;
            
            let actual_checksum = self.calculate_checksum(&content);
            
            if actual_checksum != *expected_checksum {
                return Err(SafetyError::ChecksumMismatch {
                    path: path.to_path_buf(),
                    expected: expected_checksum.clone(),
                    actual: actual_checksum,
                });
            }
        }
        
        Ok(())
    }

    /// Calculate SHA256 checksum of file content
    fn calculate_checksum(&self, content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// Get file operation statistics
    pub async fn get_statistics(&self) -> FileOperationStats {
        let read_times = self.file_read_times.lock().await;
        let checksums = self.file_checksums.lock().await;
        
        FileOperationStats {
            tracked_files: read_times.len(),
            checksums_stored: checksums.len(),
            backups_created: self.backup_manager.get_backup_count().await,
            config: self.config.clone(),
        }
    }
}

impl BackupManager {
    pub fn new(backup_dir: PathBuf, max_backups: usize) -> Self {
        Self {
            backup_dir,
            max_backups,
        }
    }

    pub async fn create_backup(&self, original_path: &Path) -> Result<PathBuf> {
        // Ensure backup directory exists
        fs::create_dir_all(&self.backup_dir)?;
        
        // Generate backup filename with timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let file_name = original_path.file_name()
            .ok_or_else(|| anyhow!("Invalid file path: {}", original_path.display()))?;
        
        let backup_filename = format!("{}_{}.backup", 
            file_name.to_string_lossy(), timestamp);
        
        let backup_path = self.backup_dir.join(backup_filename);
        
        // Copy the original file to backup location
        fs::copy(original_path, &backup_path)?;
        
        // Clean up old backups if needed
        self.cleanup_old_backups(original_path).await?;
        
        log_info!("backup", "Created backup: {} -> {}", 
                 original_path.display(), backup_path.display());
        
        Ok(backup_path)
    }

    async fn cleanup_old_backups(&self, original_path: &Path) -> Result<()> {
        let file_name = original_path.file_name()
            .ok_or_else(|| anyhow!("Invalid file path"))?
            .to_string_lossy();
        
        // Find all backups for this file
        let mut backups = Vec::new();
        
        if let Ok(entries) = fs::read_dir(&self.backup_dir) {
            for entry in entries.flatten() {
                if let Some(backup_name) = entry.file_name().to_str() {
                    if backup_name.starts_with(&format!("{}_", file_name)) && 
                       backup_name.ends_with(".backup") {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(created) = metadata.created() {
                                backups.push((entry.path(), created));
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by creation time (oldest first)
        backups.sort_by_key(|(_, created)| *created);
        
        // Remove excess backups
        while backups.len() > self.max_backups {
            let (path, _) = backups.remove(0);
            if let Err(e) = fs::remove_file(&path) {
                log_warn!("backup", "Failed to remove old backup {}: {}", path.display(), e);
            }
        }
        
        Ok(())
    }

    pub async fn get_backup_count(&self) -> usize {
        if let Ok(entries) = fs::read_dir(&self.backup_dir) {
            entries.filter_map(|e| e.ok()).count()
        } else {
            0
        }
    }

    pub async fn restore_backup(&self, backup_path: &Path, target_path: &Path) -> Result<()> {
        fs::copy(backup_path, target_path)?;
        log_info!("backup", "Restored backup: {} -> {}", 
                 backup_path.display(), target_path.display());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FileOperationStats {
    pub tracked_files: usize,
    pub checksums_stored: usize,
    pub backups_created: usize,
    pub config: SafetyConfig,
}

impl std::fmt::Display for FileOperationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "File Safety Stats: {} tracked files, {} checksums, {} backups", 
               self.tracked_files, self.checksums_stored, self.backups_created)
    }
}

/// Global file safety manager singleton
static mut GLOBAL_FILE_SAFETY: Option<FileOperationSafety> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global file safety manager
pub fn initialize_file_safety(backup_dir: PathBuf) -> Result<()> {
    INIT.call_once(|| {
        match FileOperationSafety::with_defaults(backup_dir) {
            Ok(safety_manager) => {
                unsafe {
                    GLOBAL_FILE_SAFETY = Some(safety_manager);
                }
                log_info!("file_safety", "File operation safety system initialized");
            }
            Err(e) => {
                log_error!("file_safety", "Failed to initialize file safety: {}", e);
            }
        }
    });
    Ok(())
}

/// Get reference to global file safety manager
pub fn get_file_safety() -> Option<&'static FileOperationSafety> {
    unsafe { GLOBAL_FILE_SAFETY.as_ref() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_safety_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        
        let safety = FileOperationSafety::with_defaults(backup_dir).unwrap();
        
        let test_file = temp_dir.path().join("test.txt");
        let content = "Hello, world!";
        
        // Test safe write
        safety.safe_write(&test_file, content).await.unwrap();
        assert!(test_file.exists());
        
        // Test read tracking
        safety.record_file_read(&test_file).await.unwrap();
        
        // Test modification detection
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        fs::write(&test_file, "Modified externally").unwrap();
        
        let result = safety.validate_write(&test_file, "New content").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_backup_creation() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        
        let backup_manager = BackupManager::new(backup_dir.clone(), 5);
        
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "Original content").unwrap();
        
        let backup_path = backup_manager.create_backup(&test_file).await.unwrap();
        
        assert!(backup_path.exists());
        assert_eq!(fs::read_to_string(backup_path).unwrap(), "Original content");
    }
}