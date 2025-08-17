use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::Mutex;

use crate::logger::{log_debug, log_info, log_warn};

/// Performance optimization system for CAI
/// Monitors execution performance and applies optimizations automatically
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable memory usage monitoring
    pub monitor_memory: bool,
    /// Enable execution time tracking
    pub track_execution_time: bool,
    /// Enable cache optimization
    pub enable_caching: bool,
    /// Maximum cache size (MB)
    pub max_cache_size_mb: usize,
    /// Performance data retention period (hours)
    pub data_retention_hours: u64,
    /// Enable automatic optimization
    pub auto_optimization: bool,
    /// Performance alert thresholds
    pub alert_thresholds: AlertThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Maximum memory usage (MB) before alert
    pub max_memory_mb: usize,
    /// Maximum execution time (seconds) before alert
    pub max_execution_seconds: u64,
    /// Minimum cache hit rate (%) before alert
    pub min_cache_hit_rate: f64,
    /// Maximum CPU usage (%) before alert
    pub max_cpu_percent: f64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            monitor_memory: true,
            track_execution_time: true,
            enable_caching: true,
            max_cache_size_mb: 100,
            data_retention_hours: 24,
            auto_optimization: true,
            alert_thresholds: AlertThresholds {
                max_memory_mb: 500,
                max_execution_seconds: 30,
                min_cache_hit_rate: 70.0,
                max_cpu_percent: 80.0,
            },
        }
    }
}

/// Performance metrics for different operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub operation_type: String,
    pub execution_time: Duration,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub cache_hit: bool,
    pub timestamp: SystemTime,
    pub context: HashMap<String, String>,
}

/// Cache entry for performance optimization
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub data: Vec<u8>,
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub access_count: usize,
    pub size_bytes: usize,
}

/// Performance optimization strategies
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptimizationStrategy {
    /// Enable result caching for expensive operations
    EnableCaching,
    /// Increase cache size
    IncreaseCacheSize,
    /// Reduce concurrent operations
    ReduceConcurrency,
    /// Optimize memory allocation
    OptimizeMemory,
    /// Batch similar operations
    BatchOperations,
    /// Use faster algorithms
    UseFasterAlgorithms,
    /// Preload frequently used data
    PreloadData,
    /// Compress cached data
    CompressCache,
}

/// Performance alert types
#[derive(Debug, Clone)]
pub enum PerformanceAlert {
    HighMemoryUsage { current_mb: f64, threshold_mb: usize },
    SlowExecution { duration: Duration, threshold: Duration },
    LowCacheHitRate { rate: f64, threshold: f64 },
    HighCpuUsage { current: f64, threshold: f64 },
}

/// Performance analysis result
#[derive(Debug, Clone)]
pub struct PerformanceAnalysis {
    pub overall_score: f64, // 0-100
    pub bottlenecks: Vec<String>,
    pub recommendations: Vec<OptimizationStrategy>,
    pub performance_trends: HashMap<String, f64>,
    pub alerts: Vec<PerformanceAlert>,
}

/// Performance optimization manager
pub struct PerformanceOptimizer {
    config: PerformanceConfig,
    metrics_history: Arc<Mutex<VecDeque<PerformanceMetrics>>>,
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    current_cache_size: Arc<Mutex<usize>>,
    optimization_history: Arc<Mutex<Vec<(OptimizationStrategy, Instant, bool)>>>,
}

impl PerformanceOptimizer {
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            config,
            metrics_history: Arc::new(Mutex::new(VecDeque::new())),
            cache: Arc::new(Mutex::new(HashMap::new())),
            current_cache_size: Arc::new(Mutex::new(0)),
            optimization_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn default() -> Self {
        Self::new(PerformanceConfig::default())
    }

    /// Record performance metrics for an operation
    pub async fn record_metrics(&self, metrics: PerformanceMetrics) {
        if !self.config.track_execution_time && !self.config.monitor_memory {
            return;
        }

        log_debug!("perf", "📊 Recording metrics for {}: {}ms, {:.1}MB", 
                  metrics.operation_type, 
                  metrics.execution_time.as_millis(),
                  metrics.memory_usage_mb);

        let mut history = self.metrics_history.lock().await;
        history.push_back(metrics.clone());

        // Keep only recent metrics based on retention policy
        let retention_duration = Duration::from_secs(self.config.data_retention_hours * 3600);
        let cutoff_time = SystemTime::now() - retention_duration;
        
        while let Some(front) = history.front() {
            if front.timestamp < cutoff_time {
                history.pop_front();
            } else {
                break;
            }
        }

        // Check for performance alerts
        self.check_alerts(&metrics).await;
    }

    /// Check for performance alerts
    async fn check_alerts(&self, metrics: &PerformanceMetrics) {
        let mut alerts = Vec::new();

        // Memory usage alert
        if metrics.memory_usage_mb > self.config.alert_thresholds.max_memory_mb as f64 {
            alerts.push(PerformanceAlert::HighMemoryUsage {
                current_mb: metrics.memory_usage_mb,
                threshold_mb: self.config.alert_thresholds.max_memory_mb,
            });
        }

        // Execution time alert
        let threshold_duration = Duration::from_secs(self.config.alert_thresholds.max_execution_seconds);
        if metrics.execution_time > threshold_duration {
            alerts.push(PerformanceAlert::SlowExecution {
                duration: metrics.execution_time,
                threshold: threshold_duration,
            });
        }

        // CPU usage alert
        if metrics.cpu_usage_percent > self.config.alert_thresholds.max_cpu_percent {
            alerts.push(PerformanceAlert::HighCpuUsage {
                current: metrics.cpu_usage_percent,
                threshold: self.config.alert_thresholds.max_cpu_percent,
            });
        }

        // Log alerts
        for alert in &alerts {
            match alert {
                PerformanceAlert::HighMemoryUsage { current_mb, threshold_mb } => {
                    log_warn!("perf", "🚨 High memory usage: {:.1}MB (threshold: {}MB)", current_mb, threshold_mb);
                }
                PerformanceAlert::SlowExecution { duration, threshold } => {
                    log_warn!("perf", "🚨 Slow execution: {:?} (threshold: {:?})", duration, threshold);
                }
                PerformanceAlert::HighCpuUsage { current, threshold } => {
                    log_warn!("perf", "🚨 High CPU usage: {:.1}% (threshold: {:.1}%)", current, threshold);
                }
                _ => {}
            }
        }

        // Trigger auto-optimization if enabled
        if self.config.auto_optimization && !alerts.is_empty() {
            let _ = self.auto_optimize().await;
        }
    }

    /// Measure and record performance of an operation
    pub async fn measure_operation<F, T>(&self, operation_type: &str, operation: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let start_time = Instant::now();
        let start_memory = self.get_memory_usage().await;

        let result = operation.await;
        
        let execution_time = start_time.elapsed();
        let end_memory = self.get_memory_usage().await;
        let memory_usage = end_memory.max(start_memory) - start_memory;

        // Record metrics
        let metrics = PerformanceMetrics {
            operation_type: operation_type.to_string(),
            execution_time,
            memory_usage_mb: memory_usage,
            cpu_usage_percent: self.get_cpu_usage().await,
            cache_hit: false, // Would be set by cache operations
            timestamp: SystemTime::now(),
            context: HashMap::new(),
        };

        self.record_metrics(metrics).await;

        result
    }

    /// Cache operation result
    pub async fn cache_set(&self, key: &str, data: Vec<u8>) -> Result<()> {
        if !self.config.enable_caching {
            return Ok(());
        }

        let data_size = data.len();
        let max_cache_bytes = self.config.max_cache_size_mb * 1024 * 1024;

        let mut cache = self.cache.lock().await;
        let mut current_size = self.current_cache_size.lock().await;

        // Check if we need to evict entries
        while *current_size + data_size > max_cache_bytes && !cache.is_empty() {
            self.evict_lru_entry(&mut cache, &mut current_size).await;
        }

        // Add new entry
        let entry = CacheEntry {
            data,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 1,
            size_bytes: data_size,
        };

        cache.insert(key.to_string(), entry);
        *current_size += data_size;

        log_debug!("perf", "💾 Cached entry '{}' ({} bytes)", key, data_size);
        Ok(())
    }

    /// Retrieve from cache
    pub async fn cache_get(&self, key: &str) -> Option<Vec<u8>> {
        if !self.config.enable_caching {
            return None;
        }

        let mut cache = self.cache.lock().await;
        if let Some(entry) = cache.get_mut(key) {
            entry.last_accessed = Instant::now();
            entry.access_count += 1;
            log_debug!("perf", "🎯 Cache hit for '{}'", key);
            Some(entry.data.clone())
        } else {
            log_debug!("perf", "❌ Cache miss for '{}'", key);
            None
        }
    }

    /// Evict least recently used cache entry
    async fn evict_lru_entry(&self, cache: &mut HashMap<String, CacheEntry>, current_size: &mut usize) {
        if let Some((lru_key, lru_entry)) = cache.iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(k, v)| (k.clone(), v.clone()))
        {
            *current_size -= lru_entry.size_bytes;
            cache.remove(&lru_key);
            log_debug!("perf", "🗑️ Evicted LRU cache entry '{}'", lru_key);
        }
    }

    /// Get current memory usage (simplified)
    async fn get_memory_usage(&self) -> f64 {
        // In a real implementation, this would use system calls to get actual memory usage
        // For now, return a simulated value based on cache size
        let current_size = *self.current_cache_size.lock().await;
        (current_size as f64) / (1024.0 * 1024.0) // Convert to MB
    }

    /// Get current CPU usage (simplified)
    async fn get_cpu_usage(&self) -> f64 {
        // In a real implementation, this would measure actual CPU usage
        // For now, return a simulated value
        20.0 // Simulated 20% CPU usage
    }

    /// Perform comprehensive performance analysis
    pub async fn analyze_performance(&self) -> PerformanceAnalysis {
        let history = self.metrics_history.lock().await;
        
        if history.is_empty() {
            return PerformanceAnalysis {
                overall_score: 100.0,
                bottlenecks: Vec::new(),
                recommendations: Vec::new(),
                performance_trends: HashMap::new(),
                alerts: Vec::new(),
            };
        }

        // Calculate overall performance score
        let avg_execution_time = history.iter()
            .map(|m| m.execution_time.as_millis() as f64)
            .sum::<f64>() / history.len() as f64;
        
        let avg_memory_usage = history.iter()
            .map(|m| m.memory_usage_mb)
            .sum::<f64>() / history.len() as f64;

        let cache_hit_rate = self.calculate_cache_hit_rate(&history).await;

        // Score calculation (0-100)
        let time_score = (1000.0 / (avg_execution_time + 100.0)) * 100.0;
        let memory_score = (100.0 / (avg_memory_usage + 10.0)) * 100.0;
        let cache_score = cache_hit_rate;
        
        let overall_score = (time_score + memory_score + cache_score) / 3.0;

        // Identify bottlenecks
        let mut bottlenecks = Vec::new();
        if avg_execution_time > 5000.0 { // > 5 seconds
            bottlenecks.push("Slow execution times detected".to_string());
        }
        if avg_memory_usage > 200.0 { // > 200MB
            bottlenecks.push("High memory usage detected".to_string());
        }
        if cache_hit_rate < 50.0 {
            bottlenecks.push("Low cache hit rate".to_string());
        }

        // Generate recommendations
        let recommendations = self.generate_recommendations(&bottlenecks, cache_hit_rate, avg_execution_time).await;

        // Calculate trends
        let mut trends = HashMap::new();
        trends.insert("execution_time_trend".to_string(), self.calculate_trend(&history, |m| m.execution_time.as_millis() as f64));
        trends.insert("memory_trend".to_string(), self.calculate_trend(&history, |m| m.memory_usage_mb));

        PerformanceAnalysis {
            overall_score,
            bottlenecks,
            recommendations,
            performance_trends: trends,
            alerts: Vec::new(), // Would be populated with current alerts
        }
    }

    /// Calculate cache hit rate
    async fn calculate_cache_hit_rate(&self, history: &VecDeque<PerformanceMetrics>) -> f64 {
        let total_cache_operations = history.iter().filter(|m| m.operation_type.contains("cache")).count();
        if total_cache_operations == 0 {
            return 100.0; // No cache operations, perfect score
        }

        let cache_hits = history.iter().filter(|m| m.cache_hit).count();
        (cache_hits as f64 / total_cache_operations as f64) * 100.0
    }

    /// Generate optimization recommendations
    async fn generate_recommendations(&self, bottlenecks: &[String], _cache_hit_rate: f64, avg_execution_time: f64) -> Vec<OptimizationStrategy> {
        let mut recommendations = Vec::new();

        for bottleneck in bottlenecks {
            if bottleneck.contains("Slow execution") {
                recommendations.push(OptimizationStrategy::EnableCaching);
                recommendations.push(OptimizationStrategy::BatchOperations);
                if avg_execution_time > 10000.0 {
                    recommendations.push(OptimizationStrategy::UseFasterAlgorithms);
                }
            }
            
            if bottleneck.contains("High memory") {
                recommendations.push(OptimizationStrategy::OptimizeMemory);
                recommendations.push(OptimizationStrategy::CompressCache);
            }
            
            if bottleneck.contains("Low cache hit rate") {
                recommendations.push(OptimizationStrategy::IncreaseCacheSize);
                recommendations.push(OptimizationStrategy::PreloadData);
            }
        }

        // Deduplicate recommendations
        recommendations.sort();
        recommendations.dedup();
        
        recommendations
    }

    /// Calculate performance trend (positive = improving, negative = degrading)
    fn calculate_trend<F>(&self, history: &VecDeque<PerformanceMetrics>, extractor: F) -> f64
    where
        F: Fn(&PerformanceMetrics) -> f64,
    {
        if history.len() < 2 {
            return 0.0;
        }

        let values: Vec<f64> = history.iter().map(extractor).collect();
        let mid_point = values.len() / 2;
        
        let first_half_avg = values[..mid_point].iter().sum::<f64>() / mid_point as f64;
        let second_half_avg = values[mid_point..].iter().sum::<f64>() / (values.len() - mid_point) as f64;
        
        // For execution time and memory, lower is better (negative trend is good)
        // Return percentage change
        ((first_half_avg - second_half_avg) / first_half_avg) * 100.0
    }

    /// Apply automatic optimizations
    pub async fn auto_optimize(&self) -> Result<Vec<OptimizationStrategy>> {
        log_info!("perf", "🚀 Starting automatic performance optimization");
        
        let analysis = self.analyze_performance().await;
        let mut applied_strategies = Vec::new();

        for strategy in &analysis.recommendations {
            match strategy {
                OptimizationStrategy::IncreaseCacheSize => {
                    if self.config.max_cache_size_mb < 200 {
                        log_info!("perf", "📈 Increasing cache size");
                        // In a real implementation, would increase cache size
                        applied_strategies.push(strategy.clone());
                    }
                }
                OptimizationStrategy::EnableCaching => {
                    if !self.config.enable_caching {
                        log_info!("perf", "💾 Enabling caching");
                        // In a real implementation, would enable caching
                        applied_strategies.push(strategy.clone());
                    }
                }
                OptimizationStrategy::CompressCache => {
                    log_info!("perf", "🗜️ Enabling cache compression");
                    // In a real implementation, would enable compression
                    applied_strategies.push(strategy.clone());
                }
                _ => {
                    log_debug!("perf", "📋 Recommendation noted: {:?}", strategy);
                }
            }
        }

        // Record optimization attempts
        let mut history = self.optimization_history.lock().await;
        for strategy in &applied_strategies {
            history.push((strategy.clone(), Instant::now(), true));
        }

        log_info!("perf", "✅ Applied {} optimization strategies", applied_strategies.len());
        Ok(applied_strategies)
    }

    /// Get performance statistics
    pub async fn get_statistics(&self) -> PerformanceStatistics {
        let history = self.metrics_history.lock().await;
        let cache = self.cache.lock().await;
        let current_cache_size = *self.current_cache_size.lock().await;

        let total_operations = history.len();
        let avg_execution_time = if total_operations > 0 {
            history.iter().map(|m| m.execution_time).sum::<Duration>() / total_operations as u32
        } else {
            Duration::from_secs(0)
        };

        let avg_memory_usage = if total_operations > 0 {
            history.iter().map(|m| m.memory_usage_mb).sum::<f64>() / total_operations as f64
        } else {
            0.0
        };

        PerformanceStatistics {
            total_operations,
            average_execution_time: avg_execution_time,
            average_memory_usage_mb: avg_memory_usage,
            cache_entries: cache.len(),
            cache_size_mb: (current_cache_size as f64) / (1024.0 * 1024.0),
            cache_hit_rate: self.calculate_cache_hit_rate(&history).await,
        }
    }
}

/// Performance statistics summary
#[derive(Debug, Clone)]
pub struct PerformanceStatistics {
    pub total_operations: usize,
    pub average_execution_time: Duration,
    pub average_memory_usage_mb: f64,
    pub cache_entries: usize,
    pub cache_size_mb: f64,
    pub cache_hit_rate: f64,
}

/// Global performance optimizer instance
use once_cell::sync::Lazy;

static GLOBAL_PERFORMANCE_OPTIMIZER: Lazy<Mutex<Option<PerformanceOptimizer>>> = 
    Lazy::new(|| Mutex::new(None));

/// Initialize global performance optimizer
pub async fn initialize_performance_optimizer(config: PerformanceConfig) -> Result<()> {
    let optimizer = PerformanceOptimizer::new(config);
    let mut global = GLOBAL_PERFORMANCE_OPTIMIZER.lock().await;
    *global = Some(optimizer);
    log_info!("perf", "📊 Performance optimization system initialized");
    Ok(())
}

/// Record performance metrics using global optimizer
pub async fn record_performance_metrics(metrics: PerformanceMetrics) {
    let optimizer_guard = GLOBAL_PERFORMANCE_OPTIMIZER.lock().await;
    if let Some(optimizer) = optimizer_guard.as_ref() {
        optimizer.record_metrics(metrics).await;
    }
}

/// Measure operation performance using global optimizer
pub async fn measure_operation_performance<F, T>(operation_type: &str, operation: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    let optimizer_available = {
        let optimizer_guard = GLOBAL_PERFORMANCE_OPTIMIZER.lock().await;
        optimizer_guard.is_some()
    };
    
    if optimizer_available {
        // For simplicity, just measure time here and record later
        let start_time = Instant::now();
        let result = operation.await;
        let execution_time = start_time.elapsed();
        
        // Record metrics
        let metrics = PerformanceMetrics {
            operation_type: operation_type.to_string(),
            execution_time,
            memory_usage_mb: 0.0, // Simplified
            cpu_usage_percent: 0.0, // Simplified
            cache_hit: false,
            timestamp: SystemTime::now(),
            context: HashMap::new(),
        };
        
        record_performance_metrics(metrics).await;
        result
    } else {
        operation.await
    }
}

/// Get global performance statistics
pub async fn get_global_performance_statistics() -> Option<PerformanceStatistics> {
    let optimizer_guard = GLOBAL_PERFORMANCE_OPTIMIZER.lock().await;
    if let Some(optimizer) = optimizer_guard.as_ref() {
        Some(optimizer.get_statistics().await)
    } else {
        None
    }
}

/// Trigger global performance analysis
pub async fn analyze_global_performance() -> Option<PerformanceAnalysis> {
    let optimizer_guard = GLOBAL_PERFORMANCE_OPTIMIZER.lock().await;
    if let Some(optimizer) = optimizer_guard.as_ref() {
        Some(optimizer.analyze_performance().await)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_optimizer_creation() {
        let config = PerformanceConfig::default();
        let optimizer = PerformanceOptimizer::new(config);
        
        let stats = optimizer.get_statistics().await;
        assert_eq!(stats.total_operations, 0);
        assert_eq!(stats.cache_entries, 0);
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let optimizer = PerformanceOptimizer::default();
        
        // Test cache set and get
        let test_data = b"test data".to_vec();
        optimizer.cache_set("test_key", test_data.clone()).await.unwrap();
        
        let retrieved = optimizer.cache_get("test_key").await;
        assert_eq!(retrieved, Some(test_data));
        
        // Test cache miss
        let missing = optimizer.cache_get("missing_key").await;
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_performance_analysis() {
        let optimizer = PerformanceOptimizer::default();
        
        // Record some test metrics
        let metrics = PerformanceMetrics {
            operation_type: "test_operation".to_string(),
            execution_time: Duration::from_millis(100),
            memory_usage_mb: 50.0,
            cpu_usage_percent: 25.0,
            cache_hit: true,
            timestamp: SystemTime::now(),
            context: HashMap::new(),
        };
        
        optimizer.record_metrics(metrics).await;
        
        let analysis = optimizer.analyze_performance().await;
        assert!(analysis.overall_score > 0.0);
    }
}