use colored::*;
use std::fmt;

/// Log levels with corresponding icons and colors
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Get the icon for this log level
    pub fn icon(&self) -> &'static str {
        match self {
            LogLevel::Trace => "🔍",
            LogLevel::Debug => "🐛",
            LogLevel::Info => "ℹ️",
            LogLevel::Warn => "⚠️",
            LogLevel::Error => "❌",
        }
    }

    /// Get the colored name for this log level
    pub fn colored_name(&self) -> ColoredString {
        match self {
            LogLevel::Trace => "TRACE".dimmed(),
            LogLevel::Debug => "DEBUG".blue(),
            LogLevel::Info => "INFO".green(),
            LogLevel::Warn => "WARN".yellow(),
            LogLevel::Error => "ERROR".red(),
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.colored_name())
    }
}

/// Global log level configuration
static mut LOG_LEVEL: LogLevel = LogLevel::Info;

/// Set the global log level
pub fn set_log_level(level: LogLevel) {
    unsafe {
        LOG_LEVEL = level;
    }
}

/// Get the current log level
pub fn get_log_level() -> LogLevel {
    unsafe { LOG_LEVEL }
}

/// Check if a log level should be printed
pub fn should_log(level: LogLevel) -> bool {
    level >= get_log_level()
}

/// Internal logging function
pub fn log_internal(level: LogLevel, module: &str, message: &str) {
    if should_log(level) {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
        println!("{} {} [{}] {}: {}", 
            level.icon(),
            level.colored_name(),
            timestamp.to_string().dimmed(),
            module.bright_black(),
            message
        );
    }
}

/// Logging macros for different levels
#[macro_export]
macro_rules! log_trace {
    ($module:expr, $($arg:tt)*) => {
        $crate::logger::log_internal($crate::logger::LogLevel::Trace, $module, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_debug {
    ($module:expr, $($arg:tt)*) => {
        $crate::logger::log_internal($crate::logger::LogLevel::Debug, $module, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_info {
    ($module:expr, $($arg:tt)*) => {
        $crate::logger::log_internal($crate::logger::LogLevel::Info, $module, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_warn {
    ($module:expr, $($arg:tt)*) => {
        $crate::logger::log_internal($crate::logger::LogLevel::Warn, $module, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_error {
    ($module:expr, $($arg:tt)*) => {
        $crate::logger::log_internal($crate::logger::LogLevel::Error, $module, &format!($($arg)*));
    };
}

// Export macros
pub use log_trace;
pub use log_debug;
pub use log_info;
pub use log_warn;
pub use log_error;

/// Specialized logging functions for different operations
pub mod ops {
    use super::*;

    /// Log file operations
    pub fn file_operation(operation: &str, path: &str, success: bool) {
        let icon = if success { "📁" } else { "❌" };
        let status = if success { "SUCCESS".green() } else { "FAILED".red() };
        log_internal(LogLevel::Info, "file", 
            &format!("{} {} {}: {}", icon, operation.to_uppercase(), status, path));
    }

    /// Log search operations
    pub fn search_operation(query: &str, results_count: usize) {
        log_internal(LogLevel::Info, "search", 
            &format!("🔍 Search for '{}' returned {} result(s)", query, results_count));
    }

    /// Log similarity calculations
    pub fn similarity_calculation(text1: &str, text2: &str, score: f32) {
        let truncated1 = if text1.len() > 30 { format!("{}...", &text1[..27]) } else { text1.to_string() };
        let truncated2 = if text2.len() > 30 { format!("{}...", &text2[..27]) } else { text2.to_string() };
        log_debug!("similarity", "📊 Similarity '{}' vs '{}' = {:.3}", truncated1, truncated2, score);
    }

    /// Log prompt management operations
    pub fn prompt_operation(operation: &str, file: &str, subject: &str, prompt: &str, success: bool) {
        let icon = match operation {
            "ADD" => "➕",
            "UPDATE" => "✏️",
            "SCORE" => "⭐",
            "DELETE" => "🗑️",
            _ => "📝",
        };
        let status = if success { "SUCCESS".green() } else { "FAILED".red() };
        log_internal(LogLevel::Info, "prompt", 
            &format!("{} {} {}: {} → {} → {}", icon, operation, status, file, subject, prompt));
    }

    /// Log chat operations
    pub fn chat_operation(operation: &str, details: &str) {
        let icon = match operation {
            "TASK_PLANNING" => "🧠",
            "PROMPT_ANALYSIS" => "🔍",
            "WORKFLOW_DECISION" => "⚡",
            "API_CALL" => "🌐",
            "USER_INPUT" => "💬",
            _ => "🤖",
        };
        log_info!("chat", "{} {}: {}", icon, operation, details);
    }

    /// Log initialization and startup
    pub fn startup(component: &str, status: &str) {
        let icon = match component {
            "APP" => "🚀",
            "CONFIG" => "⚙️",
            "PROMPTS" => "📚",
            "CHAT" => "💬",
            _ => "🔧",
        };
        log_info!("startup", "{} {} {}", icon, component.to_uppercase(), status);
    }

    /// Log shutdown and cleanup
    pub fn shutdown(component: &str, reason: &str) {
        let icon = match component {
            "APP" => "🛑",
            "CONFIG" => "⚙️",
            "PROMPTS" => "📚",
            "CHAT" => "💬",
            _ => "🔧",
        };
        log_info!("shutdown", "{} {} shutdown: {}", icon, component.to_uppercase(), reason);
    }

    /// Log performance metrics
    pub fn performance(operation: &str, duration_ms: u64) {
        let icon = "⏱️";
        let color = if duration_ms < 100 {
            format!("{}ms", duration_ms).green()
        } else if duration_ms < 1000 {
            format!("{}ms", duration_ms).yellow()
        } else {
            format!("{}ms", duration_ms).red()
        };
        log_debug!("perf", "{} {} completed in {}", icon, operation, color);
    }

    /// Log network operations
    pub fn network_operation(operation: &str, url: &str, status_code: Option<u16>) {
        let icon = match operation {
            "GET" => "📥",
            "POST" => "📤",
            "PUT" => "📝",
            "DELETE" => "🗑️",
            _ => "🌐",
        };
        match status_code {
            Some(code) => {
                let status_color = if code < 300 { 
                    code.to_string().green() 
                } else if code < 400 { 
                    code.to_string().yellow() 
                } else { 
                    code.to_string().red() 
                };
                log_info!("network", "{} {} {} → {}", icon, operation, url, status_color);
            }
            None => {
                log_info!("network", "{} {} {}", icon, operation, url);
            }
        }
    }

    /// Log error details with context
    pub fn error_with_context(context: &str, error: &str, details: Option<&str>) {
        let base_msg = format!("💥 {}: {}", context, error);
        if let Some(details) = details {
            log_error!("error", "{}\n   📋 Details: {}", base_msg, details);
        } else {
            log_error!("error", "{}", base_msg);
        }
    }

    /// Log MCP tool operations
    pub fn mcp_operation(operation: &str, details: &str) {
        let icon = match operation {
            "TOOL_CALL" => "🔧",
            "SERVER_START" => "🚀", 
            "SERVER_STOP" => "🛑",
            _ => "⚙️",
        };
        log_info!("mcp", "{} {} {}", icon, operation, details);
    }
}

/// Initialize logging with environment variable support
pub fn init() {
    // Check for log level environment variable
    if let Ok(level_str) = std::env::var("CAI_LOG_LEVEL") {
        let level = match level_str.to_uppercase().as_str() {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            _ => {
                eprintln!("⚠️  Invalid log level '{}', using INFO", level_str);
                LogLevel::Info
            }
        };
        set_log_level(level);
        // Print initialization message directly since the macro might not work during init
        if should_log(LogLevel::Info) {
            println!("ℹ️ INFO [{}] logger: 🔧 Log level set to {} from environment", 
                chrono::Local::now().format("%H:%M:%S%.3f"), level);
        }
    } else {
        set_log_level(LogLevel::Info);
    }

    ops::startup("LOGGER", "initialized");
}

