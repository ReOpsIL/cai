use anyhow::Result;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use crate::logger::{log_info, log_debug, log_warn};

/// Status of a scan operation
#[derive(Debug, Clone, PartialEq)]
pub enum ScanStatus {
    Idle,
    Running { 
        started_at: Instant,
        files_scanned: usize,
        total_files: Option<usize>,
        current_file: Option<String>,
    },
    Completed { 
        duration: Duration,
        files_scanned: usize,
        results: ScanResults,
    },
    Cancelled { 
        duration: Duration,
        files_scanned: usize,
    },
    Failed { 
        error: String,
        duration: Duration,
        files_scanned: usize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScanResults {
    pub rust_files: usize,
    pub lines_of_code: usize,
    pub functions: usize,
    pub structs: usize,
    pub enums: usize,
    pub traits: usize,
    pub dependencies: Vec<String>,
    pub security_issues: Vec<String>,
    pub performance_hints: Vec<String>,
}

impl Default for ScanResults {
    fn default() -> Self {
        Self {
            rust_files: 0,
            lines_of_code: 0,
            functions: 0,
            structs: 0,
            enums: 0,
            traits: 0,
            dependencies: Vec::new(),
            security_issues: Vec::new(),
            performance_hints: Vec::new(),
        }
    }
}

/// Manager for handling scan operations with status tracking and cancellation
#[derive(Debug)]
pub struct ScanManager {
    status: Arc<Mutex<ScanStatus>>,
    cancel_sender: Arc<Mutex<Option<broadcast::Sender<()>>>>,
}

impl ScanManager {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(ScanStatus::Idle)),
            cancel_sender: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Get current scan status
    pub fn get_status(&self) -> Result<ScanStatus> {
        let status = self.status.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire status lock"))?;
        Ok(status.clone())
    }
    
    /// Start a scan operation
    pub async fn start_scan(&self, scan_fn: Box<dyn Fn(broadcast::Receiver<()>) -> Result<ScanResults> + Send>) -> Result<ScanResults> {
        // Check if scan is already running
        {
            let status = self.status.lock()
                .map_err(|_| anyhow::anyhow!("Failed to acquire status lock"))?;
            if matches!(*status, ScanStatus::Running { .. }) {
                return Err(anyhow::anyhow!("Scan is already running"));
            }
        }
        
        // Create cancellation channel
        let (cancel_tx, cancel_rx) = broadcast::channel(1);
        {
            let mut sender = self.cancel_sender.lock()
                .map_err(|_| anyhow::anyhow!("Failed to acquire cancel sender lock"))?;
            *sender = Some(cancel_tx);
        }
        
        // Set status to running
        let start_time = Instant::now();
        {
            let mut status = self.status.lock()
                .map_err(|_| anyhow::anyhow!("Failed to acquire status lock"))?;
            *status = ScanStatus::Running {
                started_at: start_time,
                files_scanned: 0,
                total_files: None,
                current_file: None,
            };
        }
        
        log_info!("scan_manager", "🚀 Starting scan operation");
        
        // Execute the scan
        let result = tokio::task::spawn_blocking(move || {
            scan_fn(cancel_rx)
        }).await;
        
        let duration = start_time.elapsed();
        
        // Clear cancel sender
        {
            let mut sender = self.cancel_sender.lock()
                .map_err(|_| anyhow::anyhow!("Failed to acquire cancel sender lock"))?;
            *sender = None;
        }
        
        // Update status based on result
        match result {
            Ok(Ok(results)) => {
                let files_scanned = results.rust_files;
                let new_status = ScanStatus::Completed { duration, files_scanned, results: results.clone() };
                
                {
                    let mut status = self.status.lock()
                        .map_err(|_| anyhow::anyhow!("Failed to acquire status lock"))?;
                    *status = new_status;
                }
                
                log_info!("scan_manager", "✅ Scan completed successfully in {:?}", duration);
                Ok(results)
            }
            Ok(Err(e)) => {
                let error_msg = e.to_string();
                let is_cancelled = error_msg.contains("cancelled") || error_msg.contains("interrupted");
                
                let new_status = if is_cancelled {
                    ScanStatus::Cancelled { duration, files_scanned: 0 }
                } else {
                    ScanStatus::Failed { error: error_msg.clone(), duration, files_scanned: 0 }
                };
                
                {
                    let mut status = self.status.lock()
                        .map_err(|_| anyhow::anyhow!("Failed to acquire status lock"))?;
                    *status = new_status;
                }
                
                if is_cancelled {
                    log_info!("scan_manager", "🚫 Scan cancelled after {:?}", duration);
                } else {
                    log_warn!("scan_manager", "❌ Scan failed after {:?}: {}", duration, error_msg);
                }
                
                Err(e)
            }
            Err(e) => {
                let error_msg = format!("Task execution failed: {}", e);
                let new_status = ScanStatus::Failed { error: error_msg.clone(), duration, files_scanned: 0 };
                
                {
                    let mut status = self.status.lock()
                        .map_err(|_| anyhow::anyhow!("Failed to acquire status lock"))?;
                    *status = new_status;
                }
                
                log_warn!("scan_manager", "❌ Scan task failed after {:?}: {}", duration, error_msg);
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }
    
    /// Update scan progress
    pub fn update_progress(&self, files_scanned: usize, total_files: Option<usize>, current_file: Option<String>) -> Result<()> {
        let mut status = self.status.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire status lock"))?;
        
        if let ScanStatus::Running { started_at, .. } = *status {
            *status = ScanStatus::Running {
                started_at,
                files_scanned,
                total_files,
                current_file,
            };
            
            if files_scanned % 10 == 0 { // Log every 10 files to reduce spam
                log_debug!("scan_manager", "📊 Scan progress: {}/{:?} files", 
                          files_scanned, total_files.map(|t| t.to_string()).unwrap_or_else(|| "?".to_string()));
            }
        }
        
        Ok(())
    }
    
    /// Cancel current scan
    pub fn cancel_scan(&self) -> Result<bool> {
        let sender = self.cancel_sender.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire cancel sender lock"))?;
        
        if let Some(ref tx) = *sender {
            match tx.send(()) {
                Ok(_) => {
                    log_info!("scan_manager", "🚫 Scan cancellation signal sent");
                    Ok(true)
                }
                Err(_) => {
                    log_warn!("scan_manager", "⚠️ No active receivers for cancellation signal");
                    Ok(false)
                }
            }
        } else {
            log_warn!("scan_manager", "⚠️ No active scan to cancel");
            Ok(false)
        }
    }
    
    /// Check if scan is currently running
    pub fn is_running(&self) -> Result<bool> {
        let status = self.status.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire status lock"))?;
        Ok(matches!(*status, ScanStatus::Running { .. }))
    }
    
    /// Get scan progress percentage (0-100)
    pub fn get_progress_percentage(&self) -> Result<Option<u8>> {
        let status = self.status.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire status lock"))?;
        
        match *status {
            ScanStatus::Running { files_scanned, total_files: Some(total), .. } => {
                if total > 0 {
                    let percentage = ((files_scanned as f64 / total as f64) * 100.0) as u8;
                    Ok(Some(percentage.min(100)))
                } else {
                    Ok(None)
                }
            }
            ScanStatus::Completed { .. } => Ok(Some(100)),
            _ => Ok(None),
        }
    }
}

/// Global scan manager instance
static GLOBAL_SCAN_MANAGER: OnceLock<ScanManager> = OnceLock::new();

/// Get the global scan manager instance
pub fn get_global_scan_manager() -> &'static ScanManager {
    GLOBAL_SCAN_MANAGER.get_or_init(|| {
        log_info!("scan_manager", "Initializing global scan manager");
        ScanManager::new()
    })
}

/// Helper function to format scan status for display
impl ScanStatus {
    pub fn display_string(&self) -> String {
        match self {
            ScanStatus::Idle => "Idle - no active scan".to_string(),
            ScanStatus::Running { started_at, files_scanned, total_files, current_file } => {
                let elapsed = started_at.elapsed();
                let progress = match total_files {
                    Some(total) => format!("{}/{} files", files_scanned, total),
                    None => format!("{} files", files_scanned),
                };
                let current = current_file.as_ref()
                    .map(|f| format!(" ({})", f))
                    .unwrap_or_default();
                
                format!("Running for {:?} - {} scanned{}", elapsed, progress, current)
            }
            ScanStatus::Completed { duration, files_scanned, results } => {
                format!("Completed in {:?} - {} files, {} lines, {} functions", 
                       duration, files_scanned, results.lines_of_code, results.functions)
            }
            ScanStatus::Cancelled { duration, files_scanned } => {
                format!("Cancelled after {:?} - {} files scanned", duration, files_scanned)
            }
            ScanStatus::Failed { error, duration, files_scanned } => {
                format!("Failed after {:?} - {} files scanned: {}", duration, files_scanned, error)
            }
        }
    }
}