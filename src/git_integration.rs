use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;
use tokio::fs;
use uuid::Uuid;

/// Git workflow manager for automating version control operations
#[derive(Debug)]
pub struct GitWorkflowManager {
    /// Repository root path
    repo_path: PathBuf,
    /// Workflow tracking for context
    workflow_tracker: WorkflowTracker,
    /// Commit message analyzer for intelligent commits
    commit_analyzer: CommitAnalyzer,
    /// Branch management
    branch_manager: BranchManager,
    /// Configuration
    config: GitConfig,
}

/// Git configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    /// Default branch name for new features
    pub default_branch: String,
    /// Commit message template
    pub commit_template: String,
    /// Auto-commit settings
    pub auto_commit: AutoCommitConfig,
    /// Branch naming convention
    pub branch_naming: BranchNamingConfig,
    /// Remote settings
    pub remote: RemoteConfig,
}

/// Auto-commit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCommitConfig {
    /// Enable auto-commit
    pub enabled: bool,
    /// Files to include in auto-commits
    pub include_patterns: Vec<String>,
    /// Files to exclude from auto-commits
    pub exclude_patterns: Vec<String>,
    /// Minimum time between auto-commits (seconds)
    pub min_interval: u64,
}

/// Branch naming configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchNamingConfig {
    /// Prefix for feature branches
    pub feature_prefix: String,
    /// Prefix for bugfix branches
    pub bugfix_prefix: String,
    /// Prefix for experimental branches
    pub experiment_prefix: String,
    /// Include timestamp in branch names
    pub include_timestamp: bool,
}

/// Remote repository configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    /// Default remote name
    pub default_remote: String,
    /// Auto-push settings
    pub auto_push: bool,
    /// Create PR/MR automatically
    pub auto_create_pr: bool,
    /// PR/MR template
    pub pr_template: Option<String>,
}

/// Workflow tracking for git operations
#[derive(Debug)]
pub struct WorkflowTracker {
    active_workflows: HashMap<String, GitWorkflow>,
    completed_workflows: Vec<CompletedGitWorkflow>,
}

/// Active git workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitWorkflow {
    pub id: String,
    pub title: String,
    pub description: String,
    pub branch_name: String,
    pub base_branch: String,
    pub created_at: SystemTime,
    pub files_tracked: Vec<PathBuf>,
    pub commits: Vec<WorkflowCommit>,
    pub status: WorkflowStatus,
    pub metadata: HashMap<String, String>,
}

/// Workflow commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommit {
    pub hash: String,
    pub message: String,
    pub timestamp: SystemTime,
    pub files_changed: Vec<PathBuf>,
    pub is_auto_commit: bool,
}

/// Workflow status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Active,
    ReadyForReview,
    UnderReview,
    Completed,
    Abandoned,
}

/// Completed workflow information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedGitWorkflow {
    pub workflow: GitWorkflow,
    pub completed_at: SystemTime,
    pub final_commit_hash: Option<String>,
    pub pr_url: Option<String>,
    pub outcome: WorkflowOutcome,
}

/// Workflow completion outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowOutcome {
    Merged,
    Closed,
    Abandoned,
    Squashed,
}

/// Commit message analyzer for generating intelligent commit messages
#[derive(Debug)]
pub struct CommitAnalyzer {
    patterns: Vec<CommitPattern>,
    recent_history: Vec<String>,
}

/// Commit message pattern
#[derive(Debug, Clone)]
pub struct CommitPattern {
    pub pattern_type: CommitType,
    pub file_patterns: Vec<String>,
    pub keywords: Vec<String>,
    pub template: String,
}

/// Type of commit
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommitType {
    Feature,
    Fix,
    Refactor,
    Docs,
    Test,
    Style,
    Performance,
    Security,
    Dependency,
    Configuration,
    Initial,
    Merge,
    Revert,
}

/// Branch management
#[derive(Debug)]
pub struct BranchManager {
    current_branch: Option<String>,
    branch_history: Vec<BranchInfo>,
}

/// Branch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub created_at: SystemTime,
    pub base_branch: String,
    pub purpose: String,
    pub is_feature_branch: bool,
}

/// Git operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOperationResult {
    pub success: bool,
    pub operation: String,
    pub details: String,
    pub files_affected: Vec<PathBuf>,
    pub commit_hash: Option<String>,
    pub branch_name: Option<String>,
}

/// Pull request information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestInfo {
    pub url: String,
    pub number: u32,
    pub title: String,
    pub description: String,
    pub base_branch: String,
    pub head_branch: String,
    pub created_at: SystemTime,
}

impl GitWorkflowManager {
    /// Create a new git workflow manager
    pub async fn new(repo_path: PathBuf) -> Result<Self> {
        if !Self::is_git_repository(&repo_path).await? {
            return Err(anyhow!("Directory is not a git repository: {}", repo_path.display()));
        }

        let config = GitConfig::default();
        let workflow_tracker = WorkflowTracker::new();
        let commit_analyzer = CommitAnalyzer::new();
        let branch_manager = BranchManager::new().await?;

        Ok(Self {
            repo_path,
            workflow_tracker,
            commit_analyzer,
            branch_manager,
            config,
        })
    }

    /// Check if directory is a git repository
    async fn is_git_repository(path: &Path) -> Result<bool> {
        let git_dir = path.join(".git");
        Ok(git_dir.exists())
    }

    /// Start a new feature workflow
    pub async fn start_feature_workflow(
        &mut self,
        title: String,
        description: String,
    ) -> Result<GitWorkflow> {
        let workflow_id = Uuid::new_v4().to_string();
        let branch_name = self.generate_branch_name(&title, CommitType::Feature)?;
        
        // Create and checkout new branch
        self.create_feature_branch(&branch_name).await?;
        
        let workflow = GitWorkflow {
            id: workflow_id.clone(),
            title,
            description,
            branch_name: branch_name.clone(),
            base_branch: self.get_current_base_branch().await?,
            created_at: SystemTime::now(),
            files_tracked: Vec::new(),
            commits: Vec::new(),
            status: WorkflowStatus::Active,
            metadata: HashMap::new(),
        };

        self.workflow_tracker.active_workflows.insert(workflow_id, workflow.clone());
        self.branch_manager.register_branch(&branch_name, &workflow.base_branch, &workflow.title).await?;

        Ok(workflow)
    }

    /// Create a new feature branch
    pub async fn create_feature_branch(&self, branch_name: &str) -> Result<GitOperationResult> {
        let output = Command::new("git")
            .args(["checkout", "-b", branch_name])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            Ok(GitOperationResult {
                success: true,
                operation: "create_branch".to_string(),
                details: format!("Created and checked out branch: {}", branch_name),
                files_affected: Vec::new(),
                commit_hash: None,
                branch_name: Some(branch_name.to_string()),
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Failed to create branch {}: {}", branch_name, error))
        }
    }

    /// Auto-commit changes with intelligent commit message
    pub async fn auto_commit_changes(
        &mut self,
        workflow_id: &str,
        files: Vec<PathBuf>,
        context: &str,
    ) -> Result<GitOperationResult> {
        // Analyze changes to generate commit message
        let commit_message = self.commit_analyzer.generate_commit_message(&files, context).await?;
        
        // Stage files
        for file in &files {
            let output = Command::new("git")
                .args(["add", &file.display().to_string()])
                .current_dir(&self.repo_path)
                .output()?;
            
            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("Failed to stage file {}: {}", file.display(), error));
            }
        }

        // Create commit
        let output = Command::new("git")
            .args(["commit", "-m", &commit_message])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            let commit_hash = self.get_latest_commit_hash().await?;
            
            // Update workflow tracking
            if let Some(workflow) = self.workflow_tracker.active_workflows.get_mut(workflow_id) {
                let workflow_commit = WorkflowCommit {
                    hash: commit_hash.clone(),
                    message: commit_message.clone(),
                    timestamp: SystemTime::now(),
                    files_changed: files.clone(),
                    is_auto_commit: true,
                };
                workflow.commits.push(workflow_commit);
                workflow.files_tracked.extend(files.clone());
            }

            Ok(GitOperationResult {
                success: true,
                operation: "auto_commit".to_string(),
                details: commit_message,
                files_affected: files,
                commit_hash: Some(commit_hash),
                branch_name: Some(self.get_current_branch().await?),
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Failed to commit changes: {}", error))
        }
    }

    /// Create a pull request for the workflow
    pub async fn create_pull_request(
        &mut self,
        workflow_id: &str,
        pr_title: Option<String>,
        pr_description: Option<String>,
    ) -> Result<PullRequestInfo> {
        let workflow = self.workflow_tracker.active_workflows.get(workflow_id)
            .ok_or_else(|| anyhow!("Workflow not found: {}", workflow_id))?;

        // Push branch to remote first
        self.push_branch(&workflow.branch_name).await?;

        // Generate PR title and description if not provided
        let title = pr_title.unwrap_or_else(|| workflow.title.clone());
        let description = pr_description.unwrap_or_else(|| {
            self.generate_pr_description(workflow)
        });

        // Create PR using GitHub CLI (if available)
        let pr_info = self.create_github_pr(&workflow.branch_name, &workflow.base_branch, &title, &description).await?;

        // Update workflow status
        if let Some(workflow) = self.workflow_tracker.active_workflows.get_mut(workflow_id) {
            workflow.status = WorkflowStatus::ReadyForReview;
            workflow.metadata.insert("pr_url".to_string(), pr_info.url.clone());
            workflow.metadata.insert("pr_number".to_string(), pr_info.number.to_string());
        }

        Ok(pr_info)
    }

    /// Push current branch to remote
    async fn push_branch(&self, branch_name: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["push", "-u", &self.config.remote.default_remote, branch_name])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Failed to push branch {}: {}", branch_name, error))
        }
    }

    /// Create GitHub pull request using CLI
    async fn create_github_pr(
        &self,
        head_branch: &str,
        base_branch: &str,
        title: &str,
        description: &str,
    ) -> Result<PullRequestInfo> {
        let output = Command::new("gh")
            .args([
                "pr", "create",
                "--base", base_branch,
                "--head", head_branch,
                "--title", title,
                "--body", description,
            ])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let pr_url = output_str.trim().to_string();
            
            // Extract PR number from URL (simplified)
            let pr_number = self.extract_pr_number(&pr_url)?;

            Ok(PullRequestInfo {
                url: pr_url,
                number: pr_number,
                title: title.to_string(),
                description: description.to_string(),
                base_branch: base_branch.to_string(),
                head_branch: head_branch.to_string(),
                created_at: SystemTime::now(),
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Failed to create PR: {}", error))
        }
    }

    /// Generate PR description from workflow context
    fn generate_pr_description(&self, workflow: &GitWorkflow) -> String {
        let mut description = vec![
            format!("## Summary"),
            workflow.description.clone(),
            String::new(),
            format!("## Changes"),
        ];

        // Add commit summaries
        for commit in &workflow.commits {
            description.push(format!("- {}", commit.message));
        }

        description.push(String::new());
        description.push(format!("## Files Changed"));
        
        // Add unique files
        let mut unique_files: Vec<_> = workflow.files_tracked.iter().cloned().collect();
        unique_files.sort();
        unique_files.dedup();
        
        for file in unique_files {
            description.push(format!("- `{}`", file.display()));
        }

        description.push(String::new());
        description.push(format!("🤖 Generated with [CAI](https://github.com/anthropics/cai)"));

        description.join("\n")
    }

    /// Complete a workflow
    pub async fn complete_workflow(
        &mut self,
        workflow_id: &str,
        outcome: WorkflowOutcome,
    ) -> Result<CompletedGitWorkflow> {
        let workflow = self.workflow_tracker.active_workflows.remove(workflow_id)
            .ok_or_else(|| anyhow!("Workflow not found: {}", workflow_id))?;

        let final_commit_hash = if !workflow.commits.is_empty() {
            workflow.commits.last().map(|c| c.hash.clone())
        } else {
            None
        };

        let pr_url = workflow.metadata.get("pr_url").cloned();

        let completed_workflow = CompletedGitWorkflow {
            workflow,
            completed_at: SystemTime::now(),
            final_commit_hash,
            pr_url,
            outcome,
        };

        self.workflow_tracker.completed_workflows.push(completed_workflow.clone());

        Ok(completed_workflow)
    }

    /// Get current repository status
    pub async fn get_repository_status(&self) -> Result<RepositoryStatus> {
        let current_branch = self.get_current_branch().await?;
        let staged_files = self.get_staged_files().await?;
        let modified_files = self.get_modified_files().await?;
        let untracked_files = self.get_untracked_files().await?;
        let ahead_behind = self.get_ahead_behind_count().await?;

        Ok(RepositoryStatus {
            current_branch,
            staged_files,
            modified_files,
            untracked_files,
            ahead_count: ahead_behind.0,
            behind_count: ahead_behind.1,
            is_clean: staged_files.is_empty() && modified_files.is_empty() && untracked_files.is_empty(),
        })
    }

    /// Helper methods
    async fn get_current_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(anyhow!("Failed to get current branch"))
        }
    }

    async fn get_current_base_branch(&self) -> Result<String> {
        // Try to determine the base branch (main or master)
        let main_exists = Command::new("git")
            .args(["show-ref", "--verify", "--quiet", "refs/heads/main"])
            .current_dir(&self.repo_path)
            .status()?
            .success();

        if main_exists {
            Ok("main".to_string())
        } else {
            Ok("master".to_string())
        }
    }

    async fn get_latest_commit_hash(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(anyhow!("Failed to get latest commit hash"))
        }
    }

    async fn get_staged_files(&self) -> Result<Vec<PathBuf>> {
        let output = Command::new("git")
            .args(["diff", "--cached", "--name-only"])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            let files = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(PathBuf::from)
                .collect();
            Ok(files)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_modified_files(&self) -> Result<Vec<PathBuf>> {
        let output = Command::new("git")
            .args(["diff", "--name-only"])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            let files = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(PathBuf::from)
                .collect();
            Ok(files)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_untracked_files(&self) -> Result<Vec<PathBuf>> {
        let output = Command::new("git")
            .args(["ls-files", "--others", "--exclude-standard"])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            let files = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(PathBuf::from)
                .collect();
            Ok(files)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_ahead_behind_count(&self) -> Result<(u32, u32)> {
        let output = Command::new("git")
            .args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout).trim();
            let parts: Vec<&str> = output_str.split_whitespace().collect();
            if parts.len() == 2 {
                let ahead = parts[0].parse().unwrap_or(0);
                let behind = parts[1].parse().unwrap_or(0);
                Ok((ahead, behind))
            } else {
                Ok((0, 0))
            }
        } else {
            Ok((0, 0))
        }
    }

    fn generate_branch_name(&self, title: &str, commit_type: CommitType) -> Result<String> {
        let prefix = match commit_type {
            CommitType::Feature => &self.config.branch_naming.feature_prefix,
            CommitType::Fix => &self.config.branch_naming.bugfix_prefix,
            _ => &self.config.branch_naming.experiment_prefix,
        };

        let sanitized_title = title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();

        let branch_name = if self.config.branch_naming.include_timestamp {
            let timestamp = SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            format!("{}{}-{}", prefix, sanitized_title, timestamp)
        } else {
            format!("{}{}", prefix, sanitized_title)
        };

        Ok(branch_name)
    }

    fn extract_pr_number(&self, pr_url: &str) -> Result<u32> {
        // Extract PR number from GitHub URL (simplified)
        if let Some(captures) = regex::Regex::new(r"/pull/(\d+)")?.captures(pr_url) {
            if let Some(number_match) = captures.get(1) {
                return number_match.as_str().parse()
                    .map_err(|_| anyhow!("Invalid PR number in URL"));
            }
        }
        Err(anyhow!("Could not extract PR number from URL"))
    }

    /// Get all active workflows
    pub fn get_active_workflows(&self) -> Vec<&GitWorkflow> {
        self.workflow_tracker.active_workflows.values().collect()
    }

    /// Get workflow by ID
    pub fn get_workflow(&self, workflow_id: &str) -> Option<&GitWorkflow> {
        self.workflow_tracker.active_workflows.get(workflow_id)
    }
}

/// Repository status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryStatus {
    pub current_branch: String,
    pub staged_files: Vec<PathBuf>,
    pub modified_files: Vec<PathBuf>,
    pub untracked_files: Vec<PathBuf>,
    pub ahead_count: u32,
    pub behind_count: u32,
    pub is_clean: bool,
}

// Implementations for helper structs
impl WorkflowTracker {
    fn new() -> Self {
        Self {
            active_workflows: HashMap::new(),
            completed_workflows: Vec::new(),
        }
    }
}

impl CommitAnalyzer {
    fn new() -> Self {
        Self {
            patterns: Self::default_patterns(),
            recent_history: Vec::new(),
        }
    }

    fn default_patterns() -> Vec<CommitPattern> {
        vec![
            CommitPattern {
                pattern_type: CommitType::Feature,
                file_patterns: vec!["*.rs".to_string(), "*.py".to_string(), "*.js".to_string()],
                keywords: vec!["add".to_string(), "implement".to_string(), "create".to_string()],
                template: "feat: {}".to_string(),
            },
            CommitPattern {
                pattern_type: CommitType::Fix,
                file_patterns: vec!["*.rs".to_string(), "*.py".to_string(), "*.js".to_string()],
                keywords: vec!["fix".to_string(), "bug".to_string(), "error".to_string()],
                template: "fix: {}".to_string(),
            },
            CommitPattern {
                pattern_type: CommitType::Docs,
                file_patterns: vec!["*.md".to_string(), "*.rst".to_string(), "*.txt".to_string()],
                keywords: vec!["doc".to_string(), "readme".to_string(), "comment".to_string()],
                template: "docs: {}".to_string(),
            },
            CommitPattern {
                pattern_type: CommitType::Test,
                file_patterns: vec!["*test*.rs".to_string(), "*test*.py".to_string(), "*test*.js".to_string()],
                keywords: vec!["test".to_string(), "spec".to_string()],
                template: "test: {}".to_string(),
            },
        ]
    }

    async fn generate_commit_message(&mut self, files: &[PathBuf], context: &str) -> Result<String> {
        // Analyze files to determine commit type
        let commit_type = self.analyze_commit_type(files);
        
        // Generate message based on type and context
        let message = match commit_type {
            CommitType::Feature => format!("feat: {}", context),
            CommitType::Fix => format!("fix: {}", context),
            CommitType::Docs => format!("docs: {}", context),
            CommitType::Test => format!("test: {}", context),
            CommitType::Refactor => format!("refactor: {}", context),
            CommitType::Style => format!("style: {}", context),
            CommitType::Performance => format!("perf: {}", context),
            CommitType::Security => format!("security: {}", context),
            _ => context.to_string(),
        };

        self.recent_history.push(message.clone());
        if self.recent_history.len() > 10 {
            self.recent_history.remove(0);
        }

        Ok(message)
    }

    fn analyze_commit_type(&self, files: &[PathBuf]) -> CommitType {
        for pattern in &self.patterns {
            for file in files {
                let file_str = file.to_string_lossy();
                for file_pattern in &pattern.file_patterns {
                    if glob_match::glob_match(file_pattern, &file_str) {
                        return pattern.pattern_type.clone();
                    }
                }
            }
        }
        CommitType::Feature // Default
    }
}

impl BranchManager {
    async fn new() -> Result<Self> {
        Ok(Self {
            current_branch: None,
            branch_history: Vec::new(),
        })
    }

    async fn register_branch(&mut self, name: &str, base: &str, purpose: &str) -> Result<()> {
        let branch_info = BranchInfo {
            name: name.to_string(),
            created_at: SystemTime::now(),
            base_branch: base.to_string(),
            purpose: purpose.to_string(),
            is_feature_branch: name.contains("feature") || name.contains("feat"),
        };

        self.branch_history.push(branch_info);
        self.current_branch = Some(name.to_string());
        Ok(())
    }
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            default_branch: "main".to_string(),
            commit_template: "{type}: {description}".to_string(),
            auto_commit: AutoCommitConfig {
                enabled: true,
                include_patterns: vec!["*.rs".to_string(), "*.py".to_string(), "*.md".to_string()],
                exclude_patterns: vec!["target/".to_string(), "*.log".to_string()],
                min_interval: 300, // 5 minutes
            },
            branch_naming: BranchNamingConfig {
                feature_prefix: "feature/".to_string(),
                bugfix_prefix: "bugfix/".to_string(),
                experiment_prefix: "experiment/".to_string(),
                include_timestamp: false,
            },
            remote: RemoteConfig {
                default_remote: "origin".to_string(),
                auto_push: false,
                auto_create_pr: false,
                pr_template: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_repo() -> Result<TempDir> {
        let temp_dir = TempDir::new()?;
        
        // Initialize git repository
        Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()?;

        // Configure git user for tests
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(temp_dir.path())
            .output()?;

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(temp_dir.path())
            .output()?;

        // Create initial commit
        fs::write(temp_dir.path().join("README.md"), "# Test Repository").await?;
        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(temp_dir.path())
            .output()?;
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp_dir.path())
            .output()?;

        Ok(temp_dir)
    }

    #[tokio::test]
    async fn test_git_workflow_creation() {
        let temp_dir = setup_test_repo().await.unwrap();
        let mut git_manager = GitWorkflowManager::new(temp_dir.path().to_path_buf()).await.unwrap();

        let workflow = git_manager
            .start_feature_workflow(
                "Test Feature".to_string(),
                "Testing workflow creation".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(workflow.title, "Test Feature");
        assert_eq!(workflow.status, WorkflowStatus::Active);
        assert!(workflow.branch_name.starts_with("feature/"));
    }

    #[tokio::test]
    async fn test_commit_analysis() {
        let mut analyzer = CommitAnalyzer::new();
        
        let files = vec![PathBuf::from("src/lib.rs"), PathBuf::from("src/main.rs")];
        let message = analyzer.generate_commit_message(&files, "add new functionality").await.unwrap();
        
        assert!(message.starts_with("feat:"));
        assert!(message.contains("add new functionality"));
    }

    #[tokio::test]
    async fn test_repository_status() {
        let temp_dir = setup_test_repo().await.unwrap();
        let git_manager = GitWorkflowManager::new(temp_dir.path().to_path_buf()).await.unwrap();

        let status = git_manager.get_repository_status().await.unwrap();
        assert!(!status.current_branch.is_empty());
        assert!(status.is_clean);
    }
}