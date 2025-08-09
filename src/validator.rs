use anyhow::{anyhow, Result};
use colored::*;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidateWhen {
    Never,
    AfterAll,
}

pub struct ValidatorsRunner {
    when: ValidateWhen,
}

impl ValidatorsRunner {
    pub fn from_env() -> Self {
        let when = match std::env::var("CAI_VALIDATE").unwrap_or_default().to_lowercase().as_str() {
            "1" | "true" | "after_all" => ValidateWhen::AfterAll,
            _ => ValidateWhen::Never,
        };
        Self { when }
    }

    pub fn when(&self) -> ValidateWhen { self.when }

    pub async fn maybe_validate_project(&self) -> Result<()> {
        if self.when != ValidateWhen::AfterAll {
            return Ok(());
        }
        println!("{} Running validation checks...", "🔎".bright_blue().bold());
        // Run a minimal, safe set of validators; ignore failures gracefully but report.
        let mut had_error = false;
        if let Err(e) = run("cargo", &["fmt", "--all"]).await { report("cargo fmt", &e); had_error = true; }
        if let Err(e) = run("cargo", &["clippy", "--", "-D", "warnings"]).await { report("cargo clippy", &e); had_error = true; }
        if let Err(e) = run("cargo", &["test", "--", "--quiet"]).await { report("cargo test", &e); had_error = true; }

        if had_error {
            println!("{} One or more validators failed.", "❌".red());
        } else {
            println!("{} All validators passed.", "✅".green());
        }
        Ok(())
    }
}

async fn run(cmd: &str, args: &[&str]) -> Result<()> {
    let mut command = Command::new(cmd);
    command.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = command.output().await?;
    if !out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!("{} failed.\nSTDOUT:\n{}\nSTDERR:\n{}", cmd, stdout, stderr));
    }
    Ok(())
}

fn report(what: &str, e: &anyhow::Error) {
    println!("  {} {}: {}", "⚠️".yellow(), what.bright_white(), e);
}

