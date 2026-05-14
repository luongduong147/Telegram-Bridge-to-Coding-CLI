use std::process::Command;
use std::time::{Duration, Instant};
use std::thread;
use anyhow::{Context, Result};

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const TIMEOUT: Duration = Duration::from_secs(600);

pub fn send_keys(session: &str, keys: &str) -> Result<()> {
    Command::new("tmux")
        .args(["send-keys", "-t", session, "-l", keys])
        .status()
        .context("tmux send-keys failed")?;
    Ok(())
}

pub fn send_enter(session: &str) -> Result<()> {
    Command::new("tmux")
        .args(["send-keys", "-t", session, "Enter"])
        .status()
        .context("tmux send-keys Enter failed")?;
    Ok(())
}

pub fn capture_pane(session: &str) -> Result<String> {
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", session, "-p", "-S", "-"])
        .output()
        .context("tmux capture-pane failed")?;

    if !output.status.success() {
        anyhow::bail!("tmux capture-pane exited with {:?}", output.status.code());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn send_interrupt(session: &str) -> Result<()> {
    Command::new("tmux")
        .args(["send-keys", "-t", session, "C-c"])
        .status()
        .context("tmux send-keys C-c failed")?;
    Ok(())
}

pub fn clear_scrollback(session: &str) -> Result<()> {
    Command::new("tmux")
        .args(["send-keys", "-t", session, "-R"])
        .status()
        .context("tmux send-keys -R failed")?;
    send_keys(session, "clear")?;
    send_enter(session)?;
    Ok(())
}

pub fn send_prompt(session: &str, opencode_bin: &str, prompt: &str) -> Result<String> {
    let pid = std::process::id();
    let prompt_file = format!("/tmp/opencode_prompt_{}", pid);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let start_marker = format!("__OPC_S_{}__", ts);
    let end_marker = format!("__OPC_E_{}__", ts);

    std::fs::write(&prompt_file, prompt)
        .with_context(|| format!("failed to write prompt to {}", prompt_file))?;

    let cmd_line = format!(
        r#"echo "{}" && {} "$(cat {})" ; echo "{}""#,
        start_marker, opencode_bin, prompt_file, end_marker
    );

    send_keys(session, &cmd_line)?;
    send_enter(session)?;

    let start = Instant::now();
    loop {
        if start.elapsed() > TIMEOUT {
            let _ = send_interrupt(session);
            let _ = std::fs::remove_file(&prompt_file);
            anyhow::bail!("OpenCode timed out after 10 minutes");
        }

        let output = capture_pane(session)?;

        if let (Some(s_pos), Some(e_pos)) = (
            output.rfind(&start_marker),
            output.find(&end_marker),
        ) {
            if e_pos > s_pos {
                let content = &output[s_pos + start_marker.len()..e_pos];
                let trimmed = content.trim().to_string();
                let _ = std::fs::remove_file(&prompt_file);
                return Ok(trimmed);
            }
        }

        thread::sleep(POLL_INTERVAL);
    }
}
