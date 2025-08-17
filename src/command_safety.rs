use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::process::Command;

use crate::logger::{log_debug, log_error, log_info, log_warn};

/// Command Safety System inspired by Crush
/// Prevents dangerous command execution through comprehensive safety checks
#[derive(Debug, Clone)]
pub struct CommandSafetyAnalyzer {
    /// Commands that are completely banned
    banned_commands: HashSet<String>,
    /// Commands that are considered safe and don't require permission
    safe_commands: HashSet<String>,
    /// Complex command rules for specific patterns
    command_rules: Vec<CommandRule>,
    /// Project-specific safety context
    project_context: ProjectContext,
    /// Safety configuration
    config: CommandSafetyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSafetyConfig {
    /// Enable command safety checking
    pub enabled: bool,
    /// Allow package management commands
    pub allow_package_managers: bool,
    /// Allow network commands
    pub allow_network_commands: bool,
    /// Allow system administration commands
    pub allow_system_admin: bool,
    /// Require confirmation for medium-risk commands
    pub confirm_medium_risk: bool,
    /// Block high-risk commands entirely
    pub block_high_risk: bool,
    /// Custom safe commands to add to whitelist
    pub custom_safe_commands: Vec<String>,
    /// Custom banned commands to add to blacklist
    pub custom_banned_commands: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CommandRule {
    /// Pattern to match against command and arguments
    pub pattern: CommandPattern,
    /// Risk level of this command pattern
    pub risk_level: RiskLevel,
    /// Contexts where this command is allowed
    pub allowed_contexts: Vec<ExecutionContext>,
    /// Required confirmations before execution
    pub required_confirmations: Vec<ConfirmationType>,
    /// Description of why this command is risky
    pub risk_description: String,
}

#[derive(Debug, Clone)]
pub enum CommandPattern {
    /// Exact command name match
    Exact(String),
    /// Command with specific arguments
    CommandArgs(String, Vec<String>),
    /// Regex pattern matching
    Regex(String),
    /// Custom predicate function
    Custom(fn(&str, &[String]) -> bool),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Safe,      // Read-only operations, no system impact
    Low,       // Minor modifications in working directory
    Medium,    // Significant modifications, package installs
    High,      // System-wide changes, network operations
    Critical,  // Potentially destructive operations
}

#[derive(Debug, Clone)]
pub enum ExecutionContext {
    /// Inside a Git repository
    GitRepository,
    /// In project working directory
    ProjectDirectory,
    /// Development environment
    Development,
    /// CI/CD environment
    ContinuousIntegration,
    /// Interactive user session
    Interactive,
    /// Automated script execution
    Automated,
}

#[derive(Debug, Clone)]
pub enum ConfirmationType {
    /// Simple yes/no confirmation
    Basic,
    /// Detailed confirmation with command preview
    Detailed,
    /// Require typing command name to confirm
    TypeToConfirm,
    /// Require explicit reason from user
    ExplicitReason,
}

#[derive(Debug, Clone)]
pub struct ProjectContext {
    /// Current working directory
    pub working_directory: std::path::PathBuf,
    /// Whether we're in a Git repository
    pub is_git_repository: bool,
    /// Detected project type (rust, node, python, etc.)
    pub project_type: Option<ProjectType>,
    /// Package files found (Cargo.toml, package.json, etc.)
    pub package_files: Vec<std::path::PathBuf>,
    /// Build tools available
    pub build_tools: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Java,
    Cpp,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct CommandAnalysisResult {
    pub command: String,
    pub arguments: Vec<String>,
    pub risk_level: RiskLevel,
    pub is_allowed: bool,
    pub requires_confirmation: bool,
    pub confirmation_type: Option<ConfirmationType>,
    pub risk_description: String,
    pub suggested_alternatives: Vec<String>,
    pub safety_warnings: Vec<String>,
}

impl CommandSafetyAnalyzer {
    pub fn new(config: CommandSafetyConfig, project_context: ProjectContext) -> Self {
        let mut analyzer = Self {
            banned_commands: Self::default_banned_commands(),
            safe_commands: Self::default_safe_commands(),
            command_rules: Self::default_command_rules(),
            project_context,
            config,
        };

        // Apply custom configuration
        analyzer.apply_custom_config();
        analyzer
    }

    pub fn with_defaults(working_directory: std::path::PathBuf) -> Self {
        let project_context = ProjectContext::detect(&working_directory);
        let config = CommandSafetyConfig::default();
        Self::new(config, project_context)
    }

    /// Analyze a command for safety and determine if it should be allowed
    pub fn analyze_command(&self, command: &str, arguments: &[String]) -> CommandAnalysisResult {
        log_debug!("command_safety", "Analyzing command: {} {:?}", command, arguments);

        let mut result = CommandAnalysisResult {
            command: command.to_string(),
            arguments: arguments.to_vec(),
            risk_level: RiskLevel::Safe,
            is_allowed: true,
            requires_confirmation: false,
            confirmation_type: None,
            risk_description: String::new(),
            suggested_alternatives: Vec::new(),
            safety_warnings: Vec::new(),
        };

        if !self.config.enabled {
            log_debug!("command_safety", "Command safety disabled, allowing all commands");
            return result;
        }

        // Check if command is banned
        if self.is_banned_command(command, arguments) {
            result.is_allowed = false;
            result.risk_level = RiskLevel::Critical;
            result.risk_description = format!("Command '{}' is banned for security reasons", command);
            result.suggested_alternatives = self.suggest_alternatives(command);
            log_warn!("command_safety", "Blocked banned command: {}", command);
            return result;
        }

        // Check if command is in safe list
        if self.is_safe_command(command, arguments) {
            result.risk_level = RiskLevel::Safe;
            result.is_allowed = true;
            log_debug!("command_safety", "Allowed safe command: {}", command);
            return result;
        }

        // Apply command rules
        for rule in &self.command_rules {
            if self.matches_pattern(&rule.pattern, command, arguments) {
                result.risk_level = rule.risk_level.clone();
                result.risk_description = rule.risk_description.clone();

                // Check if context allows this command
                if !self.is_context_allowed(rule) {
                    result.is_allowed = false;
                    result.safety_warnings.push("Command not allowed in current context".to_string());
                    continue;
                }

                // Determine confirmation requirements
                result.requires_confirmation = self.requires_confirmation(&rule.risk_level);
                if result.requires_confirmation {
                    result.confirmation_type = rule.required_confirmations.first().cloned();
                }

                // Apply risk-based blocking
                if self.config.block_high_risk && result.risk_level >= RiskLevel::High {
                    result.is_allowed = false;
                    result.safety_warnings.push("High-risk commands are blocked".to_string());
                }

                break;
            }
        }

        // Generate warnings and alternatives
        self.add_safety_warnings(&mut result);
        self.add_alternatives(&mut result);

        log_info!("command_safety", "Command analysis: {} - Risk: {:?}, Allowed: {}, Confirmation: {}", 
                 command, result.risk_level, result.is_allowed, result.requires_confirmation);

        result
    }

    /// Check if a command should be executed based on safety analysis
    pub fn should_allow_execution(&self, command: &str, arguments: &[String]) -> bool {
        let analysis = self.analyze_command(command, arguments);
        analysis.is_allowed && (!analysis.requires_confirmation || self.get_user_confirmation(&analysis))
    }

    fn default_banned_commands() -> HashSet<String> {
        let banned = vec![
            // Network tools
            "curl", "wget", "nc", "netcat", "telnet", "ssh", "scp", "rsync",
            // System administration
            "sudo", "su", "doas", "mount", "umount", "fdisk", "mkfs",
            // Package managers (can be enabled via config)
            "apt", "apt-get", "yum", "dnf", "pacman", "zypper", "emerge",
            "npm", "yarn", "pip", "easy_install", "gem", "go get",
            // System modification
            "systemctl", "service", "chkconfig", "update-rc.d",
            "iptables", "ufw", "firewall-cmd",
            // Dangerous operations
            "dd", "shred", "wipe", "format", "mkfs", "fdisk",
            // Privilege escalation
            "passwd", "chpasswd", "usermod", "adduser", "deluser",
        ];

        banned.into_iter().map(String::from).collect()
    }

    fn default_safe_commands() -> HashSet<String> {
        let safe = vec![
            // File operations (read-only)
            "ls", "cat", "head", "tail", "less", "more", "file", "stat",
            "find", "locate", "which", "whereis", "type",
            // Text processing
            "grep", "awk", "sed", "sort", "uniq", "wc", "cut", "tr",
            // System information
            "ps", "top", "htop", "free", "df", "du", "uname", "whoami",
            "id", "groups", "uptime", "date", "hostname",
            // Version control (read-only)
            "git status", "git log", "git show", "git diff", "git branch",
            // Development tools (read-only)
            "cargo check", "cargo test", "npm test", "python -c",
            // Archive operations (extract only)
            "tar -tf", "tar -xf", "unzip", "gunzip", "bunzip2",
        ];

        safe.into_iter().map(String::from).collect()
    }

    fn default_command_rules() -> Vec<CommandRule> {
        vec![
            // Git operations
            CommandRule {
                pattern: CommandPattern::Exact("git".to_string()),
                risk_level: RiskLevel::Low,
                allowed_contexts: vec![ExecutionContext::GitRepository, ExecutionContext::ProjectDirectory],
                required_confirmations: vec![],
                risk_description: "Git operations can modify repository state".to_string(),
            },
            // Build commands
            CommandRule {
                pattern: CommandPattern::CommandArgs("cargo".to_string(), vec!["build".to_string()]),
                risk_level: RiskLevel::Low,
                allowed_contexts: vec![ExecutionContext::ProjectDirectory],
                required_confirmations: vec![],
                risk_description: "Cargo build is generally safe".to_string(),
            },
            // Package installation
            CommandRule {
                pattern: CommandPattern::CommandArgs("npm".to_string(), vec!["install".to_string()]),
                risk_level: RiskLevel::Medium,
                allowed_contexts: vec![ExecutionContext::ProjectDirectory],
                required_confirmations: vec![ConfirmationType::Detailed],
                risk_description: "Package installation can add dependencies".to_string(),
            },
            // File creation/modification
            CommandRule {
                pattern: CommandPattern::Exact("touch".to_string()),
                risk_level: RiskLevel::Low,
                allowed_contexts: vec![ExecutionContext::ProjectDirectory],
                required_confirmations: vec![],
                risk_description: "Creating empty files".to_string(),
            },
            // File deletion
            CommandRule {
                pattern: CommandPattern::Exact("rm".to_string()),
                risk_level: RiskLevel::High,
                allowed_contexts: vec![ExecutionContext::ProjectDirectory],
                required_confirmations: vec![ConfirmationType::TypeToConfirm],
                risk_description: "File deletion is potentially destructive".to_string(),
            },
        ]
    }

    fn is_banned_command(&self, command: &str, _arguments: &[String]) -> bool {
        self.banned_commands.contains(command)
    }

    fn is_safe_command(&self, command: &str, arguments: &[String]) -> bool {
        // Check exact command
        if self.safe_commands.contains(command) {
            return true;
        }

        // Check command with first argument
        if !arguments.is_empty() {
            let command_with_arg = format!("{} {}", command, arguments[0]);
            if self.safe_commands.contains(&command_with_arg) {
                return true;
            }
        }

        false
    }

    fn matches_pattern(&self, pattern: &CommandPattern, command: &str, arguments: &[String]) -> bool {
        match pattern {
            CommandPattern::Exact(cmd) => cmd == command,
            CommandPattern::CommandArgs(cmd, expected_args) => {
                cmd == command && arguments.starts_with(expected_args)
            }
            CommandPattern::Regex(regex) => {
                // In a full implementation, this would use the regex crate
                let full_command = format!("{} {}", command, arguments.join(" "));
                full_command.contains(regex) // Simplified for now
            }
            CommandPattern::Custom(predicate) => predicate(command, arguments),
        }
    }

    fn is_context_allowed(&self, rule: &CommandRule) -> bool {
        // For now, assume all contexts are allowed
        // In a full implementation, this would check current execution context
        true
    }

    fn requires_confirmation(&self, risk_level: &RiskLevel) -> bool {
        match risk_level {
            RiskLevel::Safe | RiskLevel::Low => false,
            RiskLevel::Medium => self.config.confirm_medium_risk,
            RiskLevel::High | RiskLevel::Critical => true,
        }
    }

    fn get_user_confirmation(&self, analysis: &CommandAnalysisResult) -> bool {
        // In a full implementation, this would show an interactive prompt
        // For now, return false to be safe
        log_warn!("command_safety", "User confirmation required for: {}", analysis.command);
        false
    }

    fn add_safety_warnings(&self, result: &mut CommandAnalysisResult) {
        match result.risk_level {
            RiskLevel::Medium => {
                result.safety_warnings.push("This command may modify your system".to_string());
            }
            RiskLevel::High => {
                result.safety_warnings.push("This command can make significant system changes".to_string());
            }
            RiskLevel::Critical => {
                result.safety_warnings.push("This command is potentially destructive".to_string());
            }
            _ => {}
        }
    }

    fn add_alternatives(&self, result: &mut CommandAnalysisResult) {
        result.suggested_alternatives = self.suggest_alternatives(&result.command);
    }

    fn suggest_alternatives(&self, command: &str) -> Vec<String> {
        match command {
            "curl" => vec!["Use MCP web tool instead".to_string()],
            "wget" => vec!["Use MCP web tool instead".to_string()],
            "rm" => vec!["Use MCP filesystem tool for safe deletion".to_string()],
            "sudo" => vec!["Run commands without elevated privileges".to_string()],
            _ => vec![],
        }
    }

    fn apply_custom_config(&mut self) {
        // Add custom safe commands
        for cmd in &self.config.custom_safe_commands {
            self.safe_commands.insert(cmd.clone());
        }

        // Add custom banned commands
        for cmd in &self.config.custom_banned_commands {
            self.banned_commands.insert(cmd.clone());
        }

        // Modify based on config flags
        if self.config.allow_package_managers {
            let package_managers = vec!["npm", "yarn", "pip", "cargo"];
            for pm in package_managers {
                self.banned_commands.remove(pm);
            }
        }

        if self.config.allow_network_commands {
            let network_commands = vec!["curl", "wget"];
            for nc in network_commands {
                self.banned_commands.remove(nc);
            }
        }
    }

    /// Update project context (call when working directory changes)
    pub fn update_project_context(&mut self, working_directory: std::path::PathBuf) {
        self.project_context = ProjectContext::detect(&working_directory);
        log_info!("command_safety", "Updated project context: {:?}", self.project_context.project_type);
    }
}

impl CommandSafetyConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn permissive() -> Self {
        Self {
            enabled: true,
            allow_package_managers: true,
            allow_network_commands: true,
            allow_system_admin: false,
            confirm_medium_risk: false,
            block_high_risk: false,
            custom_safe_commands: vec![],
            custom_banned_commands: vec![],
        }
    }

    pub fn strict() -> Self {
        Self {
            enabled: true,
            allow_package_managers: false,
            allow_network_commands: false,
            allow_system_admin: false,
            confirm_medium_risk: true,
            block_high_risk: true,
            custom_safe_commands: vec![],
            custom_banned_commands: vec![],
        }
    }
}

impl Default for CommandSafetyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allow_package_managers: false,
            allow_network_commands: false,
            allow_system_admin: false,
            confirm_medium_risk: true,
            block_high_risk: true,
            custom_safe_commands: vec![],
            custom_banned_commands: vec![],
        }
    }
}

impl ProjectContext {
    pub fn detect(working_directory: &std::path::Path) -> Self {
        let mut context = Self {
            working_directory: working_directory.to_path_buf(),
            is_git_repository: false,
            project_type: None,
            package_files: vec![],
            build_tools: vec![],
        };

        // Check for Git repository
        context.is_git_repository = working_directory.join(".git").exists();

        // Detect project type and package files
        if working_directory.join("Cargo.toml").exists() {
            context.project_type = Some(ProjectType::Rust);
            context.package_files.push(working_directory.join("Cargo.toml"));
            context.build_tools.push("cargo".to_string());
        }

        if working_directory.join("package.json").exists() {
            context.project_type = Some(ProjectType::Node);
            context.package_files.push(working_directory.join("package.json"));
            context.build_tools.extend(vec!["npm".to_string(), "yarn".to_string()]);
        }

        if working_directory.join("requirements.txt").exists() || 
           working_directory.join("pyproject.toml").exists() {
            context.project_type = Some(ProjectType::Python);
            if working_directory.join("requirements.txt").exists() {
                context.package_files.push(working_directory.join("requirements.txt"));
            }
            if working_directory.join("pyproject.toml").exists() {
                context.package_files.push(working_directory.join("pyproject.toml"));
            }
            context.build_tools.push("pip".to_string());
        }

        context
    }
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Safe => write!(f, "Safe"),
            RiskLevel::Low => write!(f, "Low"),
            RiskLevel::Medium => write!(f, "Medium"),
            RiskLevel::High => write!(f, "High"),
            RiskLevel::Critical => write!(f, "Critical"),
        }
    }
}

/// Global command safety analyzer singleton
static mut GLOBAL_COMMAND_SAFETY: Option<CommandSafetyAnalyzer> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global command safety analyzer
pub fn initialize_command_safety(working_directory: std::path::PathBuf) {
    INIT.call_once(|| {
        let analyzer = CommandSafetyAnalyzer::with_defaults(working_directory);
        unsafe {
            GLOBAL_COMMAND_SAFETY = Some(analyzer);
        }
        log_info!("command_safety", "Command safety system initialized");
    });
}

/// Get reference to global command safety analyzer
pub fn get_command_safety() -> Option<&'static CommandSafetyAnalyzer> {
    unsafe { GLOBAL_COMMAND_SAFETY.as_ref() }
}

/// Get mutable reference to global command safety analyzer
pub fn get_command_safety_mut() -> Option<&'static mut CommandSafetyAnalyzer> {
    unsafe { GLOBAL_COMMAND_SAFETY.as_mut() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_command_safety_basic() {
        let temp_dir = TempDir::new().unwrap();
        let analyzer = CommandSafetyAnalyzer::with_defaults(temp_dir.path().to_path_buf());

        // Test safe command
        let result = analyzer.analyze_command("ls", &[]);
        assert!(result.is_allowed);
        assert_eq!(result.risk_level, RiskLevel::Safe);

        // Test banned command
        let result = analyzer.analyze_command("sudo", &["rm".to_string(), "-rf".to_string(), "/".to_string()]);
        assert!(!result.is_allowed);
        assert_eq!(result.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_project_context_detection() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create a Rust project
        std::fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        
        let context = ProjectContext::detect(temp_dir.path());
        assert_eq!(context.project_type, Some(ProjectType::Rust));
        assert!(context.build_tools.contains(&"cargo".to_string()));
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Safe < RiskLevel::Low);
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }
}