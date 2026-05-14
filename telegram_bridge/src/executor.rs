use std::process::Command;
use std::time::Duration;
use anyhow::{Context, Result};

const CMD_TIMEOUT: Duration = Duration::from_secs(600);

pub fn run_opencode(
    opencode_bin: &str,
    workdir: &str,
    prompt: &str,
    continue_session: bool,
) -> Result<String> {
    let mut child = Command::new(opencode_bin)
        .arg("run")
        .arg("--dir")
        .arg(workdir)
        .args(if continue_session { vec!["-c"] } else { vec![] })
        .arg(prompt)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn opencode")?;

    let now = std::time::Instant::now();
    let timeout = CMD_TIMEOUT;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().ok();
                let stdout = output
                    .as_ref()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_default();
                let stderr = output
                    .as_ref()
                    .map(|o| String::from_utf8_lossy(&o.stderr).trim().to_string())
                    .unwrap_or_default();

                if !status.success() {
                    anyhow::bail!("opencode run failed (exit: {:?}): {}", status.code(), stderr);
                }
                return Ok(stdout);
            }
            Ok(None) => {
                if now.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    anyhow::bail!("OpenCode timed out after 10 minutes");
                }
                std::thread::sleep(Duration::from_millis(500));
            }
            Err(e) => {
                anyhow::bail!("Error waiting for opencode: {}", e);
            }
        }
    }
}
