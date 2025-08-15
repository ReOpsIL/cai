use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use cai::scan_manager::*;

#[tokio::test]
async fn test_scan_manager_creation() {
    let manager = ScanManager::new();
    
    let status = manager.get_status().unwrap();
    assert!(matches!(status, ScanStatus::Idle));
    
    assert!(!manager.is_running().unwrap());
    
    let progress = manager.get_progress_percentage().unwrap();
    assert!(progress.is_none());
}

#[tokio::test]
async fn test_scan_execution_success() -> Result<()> {
    let manager = ScanManager::new();
    
    let test_results = ScanResults {
        rust_files: 5,
        lines_of_code: 1000,
        functions: 50,
        structs: 10,
        enums: 3,
        traits: 2,
        dependencies: vec!["tokio".to_string(), "anyhow".to_string()],
        security_issues: vec![],
        performance_hints: vec!["Consider using Arc for shared data".to_string()],
    };
    
    let expected_results = test_results.clone();
    let scan_fn = Box::new(move |_cancel_rx: broadcast::Receiver<()>| -> Result<ScanResults> {
        // Simulate some work
        std::thread::sleep(Duration::from_millis(10));
        Ok(test_results)
    });
    
    let results = manager.start_scan(scan_fn).await?;
    
    assert_eq!(results.rust_files, expected_results.rust_files);
    assert_eq!(results.lines_of_code, expected_results.lines_of_code);
    assert_eq!(results.functions, expected_results.functions);
    assert_eq!(results.dependencies, expected_results.dependencies);
    
    let status = manager.get_status()?;
    match status {
        ScanStatus::Completed { files_scanned, results: final_results, .. } => {
            assert_eq!(files_scanned, 5);
            assert_eq!(final_results.rust_files, 5);
        }
        _ => panic!("Expected Completed status, got {:?}", status)
    }
    
    Ok(())
}

#[tokio::test]
async fn test_scan_cancellation() -> Result<()> {
    let manager = ScanManager::new();
    
    let scan_fn = Box::new(move |mut cancel_rx: broadcast::Receiver<()>| -> Result<ScanResults> {
        // Simulate long-running operation that checks for cancellation
        for i in 0..100 {
            if let Ok(_) = cancel_rx.try_recv() {
                return Err(anyhow::anyhow!("Scan was cancelled at iteration {}", i));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(ScanResults::default())
    });
    
    // Start scan in background
    let manager_clone = Arc::new(manager);
    let scan_manager = manager_clone.clone();
    let scan_handle = tokio::spawn(async move {
        scan_manager.start_scan(scan_fn).await
    });
    
    // Wait a bit then cancel
    tokio::time::sleep(Duration::from_millis(10)).await;
    let cancel_result = manager_clone.cancel_scan()?;
    assert!(cancel_result);
    
    // Wait for scan to complete
    let result = scan_handle.await.unwrap();
    assert!(result.is_err());
    
    let status = manager_clone.get_status()?;
    match status {
        ScanStatus::Failed { error, .. } => {
            assert!(error.contains("cancelled"));
        }
        _ => panic!("Expected Failed status due to cancellation, got {:?}", status)
    }
    
    Ok(())
}

#[tokio::test]
async fn test_scan_progress_tracking() -> Result<()> {
    let manager = ScanManager::new();
    
    let scan_fn = Box::new(move |cancel_rx: broadcast::Receiver<()>| -> Result<ScanResults> {
        // Simulate scan with progress updates (this would be called from the scan function)
        Ok(ScanResults {
            rust_files: 10,
            lines_of_code: 2000,
            functions: 100,
            structs: 20,
            enums: 5,
            traits: 4,
            dependencies: vec!["tokio".to_string()],
            security_issues: vec![],
            performance_hints: vec![],
        })
    });
    
    let manager_clone = Arc::new(manager);
    let progress_manager = manager_clone.clone();
    let scan_manager = manager_clone.clone();
    
    let scan_handle = tokio::spawn(async move {
        scan_manager.start_scan(scan_fn).await
    });
    
    // Simulate progress updates
    tokio::time::sleep(Duration::from_millis(5)).await;
    progress_manager.update_progress(5, Some(10), Some("src/main.rs".to_string()))?;
    
    let status = progress_manager.get_status()?;
    match status {
        ScanStatus::Running { files_scanned, total_files, current_file, .. } => {
            assert_eq!(files_scanned, 5);
            assert_eq!(total_files, Some(10));
            assert_eq!(current_file, Some("src/main.rs".to_string()));
        }
        _ => {} // Might have completed already in fast execution
    }
    
    let progress = progress_manager.get_progress_percentage()?;
    if let Some(percentage) = progress {
        assert_eq!(percentage, 50);
    }
    
    scan_handle.await.unwrap()?;
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_scan_rejection() -> Result<()> {
    let manager = Arc::new(ScanManager::new());
    
    let long_scan = Box::new(move |_cancel_rx: broadcast::Receiver<()>| -> Result<ScanResults> {
        std::thread::sleep(Duration::from_millis(50));
        Ok(ScanResults::default())
    });
    
    let manager1 = manager.clone();
    let manager2 = manager.clone();
    
    // Start first scan
    let handle1 = tokio::spawn(async move {
        manager1.start_scan(long_scan).await
    });
    
    // Try to start second scan immediately
    tokio::time::sleep(Duration::from_millis(5)).await;
    let quick_scan = Box::new(move |_cancel_rx: broadcast::Receiver<()>| -> Result<ScanResults> {
        Ok(ScanResults::default())
    });
    
    let result2 = manager2.start_scan(quick_scan).await;
    assert!(result2.is_err());
    assert!(result2.unwrap_err().to_string().contains("already running"));
    
    // First scan should complete successfully
    let result1 = handle1.await.unwrap();
    assert!(result1.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_scan_failure_handling() -> Result<()> {
    let manager = ScanManager::new();
    
    let failing_scan = Box::new(move |_cancel_rx: broadcast::Receiver<()>| -> Result<ScanResults> {
        Err(anyhow::anyhow!("Simulated scan failure"))
    });
    
    let result = manager.start_scan(failing_scan).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Simulated scan failure"));
    
    let status = manager.get_status()?;
    match status {
        ScanStatus::Failed { error, .. } => {
            assert!(error.contains("Simulated scan failure"));
        }
        _ => panic!("Expected Failed status, got {:?}", status)
    }
    
    Ok(())
}

#[tokio::test]
async fn test_global_scan_manager() -> Result<()> {
    let global_manager = get_global_scan_manager();
    
    let status = global_manager.get_status()?;
    assert!(matches!(status, ScanStatus::Idle));
    
    let test_scan = Box::new(move |_cancel_rx: broadcast::Receiver<()>| -> Result<ScanResults> {
        Ok(ScanResults {
            rust_files: 1,
            lines_of_code: 100,
            functions: 5,
            structs: 1,
            enums: 0,
            traits: 0,
            dependencies: vec![],
            security_issues: vec![],
            performance_hints: vec![],
        })
    });
    
    let result = global_manager.start_scan(test_scan).await?;
    assert_eq!(result.rust_files, 1);
    assert_eq!(result.lines_of_code, 100);
    
    // Should be accessible from another reference
    let same_manager = get_global_scan_manager();
    let status = same_manager.get_status()?;
    match status {
        ScanStatus::Completed { .. } => {} // Expected
        _ => panic!("Expected Completed status from global instance")
    }
    
    Ok(())
}

#[test]
fn test_scan_status_display() {
    let idle_status = ScanStatus::Idle;
    assert_eq!(idle_status.display_string(), "Idle - no active scan");
    
    let completed_status = ScanStatus::Completed {
        duration: Duration::from_secs(10),
        files_scanned: 50,
        results: ScanResults {
            rust_files: 50,
            lines_of_code: 5000,
            functions: 250,
            structs: 30,
            enums: 5,
            traits: 8,
            dependencies: vec![],
            security_issues: vec![],
            performance_hints: vec![],
        }
    };
    
    let display = completed_status.display_string();
    assert!(display.contains("Completed in 10s"));
    assert!(display.contains("50 files"));
    assert!(display.contains("5000 lines"));
    assert!(display.contains("250 functions"));
}

#[test]
fn test_scan_results_default() {
    let results = ScanResults::default();
    assert_eq!(results.rust_files, 0);
    assert_eq!(results.lines_of_code, 0);
    assert_eq!(results.functions, 0);
    assert_eq!(results.structs, 0);
    assert_eq!(results.enums, 0);
    assert_eq!(results.traits, 0);
    assert!(results.dependencies.is_empty());
    assert!(results.security_issues.is_empty());
    assert!(results.performance_hints.is_empty());
}

#[tokio::test]
async fn test_cancel_nonexistent_scan() -> Result<()> {
    let manager = ScanManager::new();
    
    let result = manager.cancel_scan()?;
    assert!(!result); // Should return false for no active scan
    
    Ok(())
}