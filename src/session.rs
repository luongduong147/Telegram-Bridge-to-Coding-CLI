use std::process::Command;
use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct TmuxSession {
    pub name: String,
    pub workdir: String,
}

impl TmuxSession {
    pub fn new(name: &str, workdir: &str) -> Self {
        Self {
            name: name.to_string(),
            workdir: workdir.to_string(),
        }
    }

    pub fn create(&self) -> Result<()> {
        let status = Command::new("tmux")
            .args([
                "new-session",
                "-d",
                "-s",
                &self.name,
                "-c",
                &self.workdir,
            ])
            .status()
            .context("Failed to run tmux new-session")?;

        if !status.success() {
            anyhow::bail!("tmux new-session failed (exit: {:?})", status.code());
        }

        tracing::info!(session = %self.name, "Created tmux session");
        Ok(())
    }

    pub fn exists(&self) -> Result<bool> {
        let status = Command::new("tmux")
            .args(["has-session", "-t", &self.name])
            .status()
            .context("Failed to run tmux has-session")?;

        Ok(status.success())
    }

    pub fn kill(&self) -> Result<()> {
        let status = Command::new("tmux")
            .args(["kill-session", "-t", &self.name])
            .status()
            .context("Failed to run tmux kill-session")?;

        if !status.success() {
            anyhow::bail!("tmux kill-session failed (exit: {:?})", status.code());
        }

        tracing::info!(session = %self.name, "Killed tmux session");
        Ok(())
    }

    pub fn ensure_running(&self) -> Result<()> {
        if !self.exists()? {
            self.create()?;
        }
        Ok(())
    }
}
