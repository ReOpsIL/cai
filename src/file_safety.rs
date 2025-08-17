use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;
use tokio::sync::Mutex;
use uuid::Uuid;

/// File safety manager implementing read-before-edit validation and atomic operations
#[derive(Debug)]
pub struct FileSafetyManager {
    /// Track file modification times
    file_states: HashMap<PathBuf, FileState>,
    /// Lock management for concurrent access
    file_locks: HashMap<PathBuf, Arc<Mutex<()>>>,
    /// Read-before-edit validation
    read_history: HashMap<PathBuf, ReadRecord>,
    /// Temporary directory for atomic operations
    temp_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    pub last_modified: SystemTime,
    pub content_hash: u64,
    pub last_read_by_cai: Option<SystemTime>,
    pub is_locked: bool,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct ReadRecord {
    pub timestamp: SystemTime,
    pub content_hash: u64,
    pub content_size: u64,
    pub read_id: String,
}

#[derive(Debug, Clone)]
pub struct SafeWriteOperation {
    pub operation_id: String,
    pub target_path: PathBuf,
    pub temp_path: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub original_content_hash: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum FileValidationError {
    FileModifiedSinceRead { 
        path: PathBuf, 
        last_read: SystemTime, 
        current_modified: SystemTime 
    },
    ContentHashMismatch { 
        path: PathBuf, 
        expected_hash: u64, 
        actual_hash: u64 
    },
    FileNotFound { path: PathBuf },
    FileLocked { path: PathBuf },
    ConcurrentModification { path: PathBuf },
    PermissionDenied { path: PathBuf },
    InvalidOperation { reason: String },
}

impl FileSafetyManager {
    pub async fn new() -> Result<Self> {
        let temp_dir = std::env::temp_dir().join("cai-file-safety");
        fs::create_dir_all(&temp_dir).await?;

        Ok(Self {
            file_states: HashMap::new(),
            file_locks: HashMap::new(),
            read_history: HashMap::new(),
            temp_dir,
        })
    }

    /// Safely read a file with tracking for later validation
    pub async fn safe_read_file(&mut self, path: &Path) -> Result<String> {
        // Get or create file lock
        let lock = self.get_or_create_lock(path).clone();
        let _guard = lock.lock().await;

        // Read file content
        let content = match fs::read_to_string(path).await {
            Ok(content) => content,
            Err(e) => return Err(anyhow!("Failed to read file {}: {}", path.display(), e)),
        };

        // Get file metadata
        let metadata = fs::metadata(path).await
            .map_err(|e| anyhow!("Failed to get metadata for {}: {}", path.display(), e))?;

        let modified_time = metadata.modified()
            .map_err(|e| anyhow!("Failed to get modification time for {}: {}", path.display(), e))?;

        // Calculate content hash
        let content_hash = self.calculate_hash(&content);

        // Record the read operation
        let read_record = ReadRecord {
            timestamp: SystemTime::now(),
            content_hash,
            content_size: content.len() as u64,
            read_id: Uuid::new_v4().to_string(),
        };

        self.read_history.insert(path.to_path_buf(), read_record);

        // Update file state
        let file_state = FileState {
            last_modified: modified_time,
            content_hash,
            last_read_by_cai: Some(SystemTime::now()),
            is_locked: false,
            size: metadata.len(),
        };

        self.file_states.insert(path.to_path_buf(), file_state);

        Ok(content)
    }

    /// Safely write a file with validation and atomic operations
    pub async fn safe_write_file(
        &mut self,
        path: &Path,
        old_content: &str,
        new_content: &str,
    ) -> Result<SafeWriteOperation> {
        // Validate file state before writing
        self.validate_file_state_for_write(path, old_content).await?;

        // Get file lock
        let lock = self.get_or_create_lock(path).clone();
        let _guard = lock.lock().await;

        // Create atomic write operation
        let operation = self.prepare_atomic_write(path, old_content, new_content).await?;

        // Perform the atomic write
        self.execute_atomic_write(&operation, new_content).await?;

        // Update tracking
        self.update_file_state_after_write(path, new_content).await?;

        Ok(operation)
    }

    /// Create a file safely (only if it doesn't exist)
    pub async fn safe_create_file(
        &mut self,
        path: &Path,
        content: &str,
    ) -> Result<SafeWriteOperation> {
        // Check if file already exists
        if path.exists() {
            return Err(anyhow!("File already exists: {}", path.display()));
        }

        // Get file lock
        let lock = self.get_or_create_lock(path).clone();
        let _guard = lock.lock().await;

        // Prepare atomic create operation
        let operation = self.prepare_atomic_create(path, content).await?;

        // Execute the atomic create
        self.execute_atomic_write(&operation, content).await?;

        // Update tracking
        self.update_file_state_after_write(path, content).await?;

        Ok(operation)
    }

    /// Validate file state before writing
    async fn validate_file_state_for_write(
        &self,
        path: &Path,
        expected_content: &str,
    ) -> Result<(), FileValidationError> {
        // Check if we have a read record for this file
        let read_record = self.read_history.get(path)
            .ok_or_else(|| FileValidationError::InvalidOperation {
                reason: format!("File {} must be read before editing", path.display())
            })?;

        // Check if file still exists
        if !path.exists() {
            return Err(FileValidationError::FileNotFound { 
                path: path.to_path_buf() 
            });
        }

        // Get current file metadata
        let metadata = fs::metadata(path).await
            .map_err(|_| FileValidationError::FileNotFound { 
                path: path.to_path_buf() 
            })?;

        let current_modified = metadata.modified()
            .map_err(|_| FileValidationError::ConcurrentModification { 
                path: path.to_path_buf() 
            })?;

        // Check if file was modified since our last read
        if let Some(file_state) = self.file_states.get(path) {
            if current_modified > file_state.last_modified {
                return Err(FileValidationError::FileModifiedSinceRead {
                    path: path.to_path_buf(),
                    last_read: read_record.timestamp,
                    current_modified,
                });
            }
        }

        // Verify content hasn't changed by reading current content
        let current_content = fs::read_to_string(path).await
            .map_err(|_| FileValidationError::ConcurrentModification { 
                path: path.to_path_buf() 
            })?;

        let current_hash = self.calculate_hash(&current_content);
        if current_hash != read_record.content_hash {
            return Err(FileValidationError::ContentHashMismatch {
                path: path.to_path_buf(),
                expected_hash: read_record.content_hash,
                actual_hash: current_hash,
            });
        }

        // Verify expected content matches what we read
        if current_content != expected_content {
            return Err(FileValidationError::ContentHashMismatch {
                path: path.to_path_buf(),
                expected_hash: self.calculate_hash(expected_content),
                actual_hash: current_hash,
            });
        }

        Ok(())
    }

    /// Prepare atomic write operation
    async fn prepare_atomic_write(
        &self,
        path: &Path,
        old_content: &str,
        new_content: &str,
    ) -> Result<SafeWriteOperation> {
        let operation_id = Uuid::new_v4().to_string();
        let temp_path = self.temp_dir.join(format!("{}.tmp", operation_id));
        
        // Create backup if file exists
        let backup_path = if path.exists() {
            let backup_path = self.temp_dir.join(format!("{}.backup", operation_id));
            fs::copy(path, &backup_path).await?;
            Some(backup_path)
        } else {
            None
        };

        let original_content_hash = if path.exists() {
            Some(self.calculate_hash(old_content))
        } else {
            None
        };

        Ok(SafeWriteOperation {
            operation_id,
            target_path: path.to_path_buf(),
            temp_path,
            backup_path,
            original_content_hash,
        })
    }

    /// Prepare atomic create operation
    async fn prepare_atomic_create(
        &self,
        path: &Path,
        content: &str,
    ) -> Result<SafeWriteOperation> {
        let operation_id = Uuid::new_v4().to_string();
        let temp_path = self.temp_dir.join(format!("{}.tmp", operation_id));

        Ok(SafeWriteOperation {
            operation_id,
            target_path: path.to_path_buf(),
            temp_path,
            backup_path: None,
            original_content_hash: None,
        })
    }

    /// Execute atomic write operation
    async fn execute_atomic_write(
        &self,
        operation: &SafeWriteOperation,
        content: &str,
    ) -> Result<()> {
        // Write to temporary file first
        fs::write(&operation.temp_path, content).await
            .map_err(|e| anyhow!("Failed to write temporary file: {}", e))?;

        // Ensure parent directory exists
        if let Some(parent) = operation.target_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| anyhow!("Failed to create parent directory: {}", e))?;
        }

        // Atomic move from temp to target
        fs::rename(&operation.temp_path, &operation.target_path).await
            .map_err(|e| anyhow!("Failed to move temporary file to target: {}", e))?;

        Ok(())
    }

    /// Update file state after successful write
    async fn update_file_state_after_write(
        &mut self,
        path: &Path,
        new_content: &str,
    ) -> Result<()> {
        let metadata = fs::metadata(path).await?;
        let modified_time = metadata.modified()?;
        let content_hash = self.calculate_hash(new_content);

        let file_state = FileState {
            last_modified: modified_time,
            content_hash,
            last_read_by_cai: Some(SystemTime::now()),
            is_locked: false,
            size: metadata.len(),
        };

        self.file_states.insert(path.to_path_buf(), file_state);

        // Update read history with new content
        let read_record = ReadRecord {
            timestamp: SystemTime::now(),
            content_hash,
            content_size: new_content.len() as u64,
            read_id: Uuid::new_v4().to_string(),
        };

        self.read_history.insert(path.to_path_buf(), read_record);

        Ok(())
    }

    /// Rollback an atomic operation
    pub async fn rollback_operation(&mut self, operation: &SafeWriteOperation) -> Result<()> {
        // If backup exists, restore it
        if let Some(backup_path) = &operation.backup_path {
            if backup_path.exists() {
                fs::copy(backup_path, &operation.target_path).await?;
                fs::remove_file(backup_path).await?;
            }
        } else {
            // If no backup, remove the created file
            if operation.target_path.exists() {
                fs::remove_file(&operation.target_path).await?;
            }
        }

        // Clean up temporary files
        if operation.temp_path.exists() {
            fs::remove_file(&operation.temp_path).await?;
        }

        // Remove from tracking
        self.file_states.remove(&operation.target_path);
        self.read_history.remove(&operation.target_path);

        Ok(())
    }

    /// Get or create a file lock
    fn get_or_create_lock(&mut self, path: &Path) -> &Arc<Mutex<()>> {
        self.file_locks
            .entry(path.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(())))
    }

    /// Calculate hash for content
    fn calculate_hash(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Check if file is safe to read
    pub async fn is_safe_to_read(&self, path: &Path) -> Result<bool> {
        if !path.exists() {
            return Ok(false);
        }

        // Check if file is locked
        if let Some(file_state) = self.file_states.get(path) {
            if file_state.is_locked {
                return Ok(false);
            }
        }

        // Check permissions
        let metadata = fs::metadata(path).await?;
        Ok(metadata.permissions().readonly() == false)
    }

    /// Get file state information
    pub fn get_file_state(&self, path: &Path) -> Option<&FileState> {
        self.file_states.get(path)
    }

    /// Clear tracking for a file
    pub fn clear_file_tracking(&mut self, path: &Path) {
        self.file_states.remove(path);
        self.read_history.remove(path);
        self.file_locks.remove(path);
    }

    /// Clean up old temporary files
    pub async fn cleanup_temp_files(&self) -> Result<()> {
        let mut entries = fs::read_dir(&self.temp_dir).await?;
        let now = SystemTime::now();
        let one_hour_ago = now.checked_sub(std::time::Duration::from_secs(3600))
            .unwrap_or(now);

        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if let Ok(created) = metadata.created() {
                if created < one_hour_ago {
                    let _ = fs::remove_file(entry.path()).await;
                }
            }
        }

        Ok(())
    }
}

impl std::fmt::Display for FileValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileValidationError::FileModifiedSinceRead { path, .. } => {
                write!(f, "File {} was modified since last read", path.display())
            }
            FileValidationError::ContentHashMismatch { path, .. } => {
                write!(f, "Content of {} has changed unexpectedly", path.display())
            }
            FileValidationError::FileNotFound { path } => {
                write!(f, "File {} not found", path.display())
            }
            FileValidationError::FileLocked { path } => {
                write!(f, "File {} is locked", path.display())
            }
            FileValidationError::ConcurrentModification { path } => {
                write!(f, "File {} was modified concurrently", path.display())
            }
            FileValidationError::PermissionDenied { path } => {
                write!(f, "Permission denied for file {}", path.display())
            }
            FileValidationError::InvalidOperation { reason } => {
                write!(f, "Invalid operation: {}", reason)
            }
        }
    }
}

impl std::error::Error for FileValidationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{NamedTempFile, TempDir};

    #[tokio::test]
    async fn test_safe_read_and_write() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        
        // Create initial file
        fs::write(&test_file, "initial content").await.unwrap();
        
        let mut manager = FileSafetyManager::new().await.unwrap();
        
        // Read the file
        let content = manager.safe_read_file(&test_file).await.unwrap();
        assert_eq!(content, "initial content");
        
        // Write new content
        let operation = manager.safe_write_file(
            &test_file,
            "initial content",
            "updated content"
        ).await.unwrap();
        
        // Verify content was written
        let new_content = fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(new_content, "updated content");
    }

    #[tokio::test]
    async fn test_concurrent_modification_detection() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        
        // Create initial file
        fs::write(&test_file, "initial content").await.unwrap();
        
        let mut manager = FileSafetyManager::new().await.unwrap();
        
        // Read the file
        let _content = manager.safe_read_file(&test_file).await.unwrap();
        
        // Simulate external modification
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        fs::write(&test_file, "externally modified").await.unwrap();
        
        // Attempt to write should fail
        let result = manager.safe_write_file(
            &test_file,
            "initial content",
            "our update"
        ).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_safe_create_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("new_file.txt");
        
        let mut manager = FileSafetyManager::new().await.unwrap();
        
        // Create new file
        let operation = manager.safe_create_file(&test_file, "new content").await.unwrap();
        
        // Verify file was created
        assert!(test_file.exists());
        let content = fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "new content");
        
        // Attempt to create again should fail
        let result = manager.safe_create_file(&test_file, "different content").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rollback_operation() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        
        // Create initial file
        fs::write(&test_file, "original content").await.unwrap();
        
        let mut manager = FileSafetyManager::new().await.unwrap();
        
        // Read the file
        let _content = manager.safe_read_file(&test_file).await.unwrap();
        
        // Write new content
        let operation = manager.safe_write_file(
            &test_file,
            "original content",
            "modified content"
        ).await.unwrap();
        
        // Verify modification
        let content = fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "modified content");
        
        // Rollback
        manager.rollback_operation(&operation).await.unwrap();
        
        // Verify rollback
        let content = fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "original content");
    }
}