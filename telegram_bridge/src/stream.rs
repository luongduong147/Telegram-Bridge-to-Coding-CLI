use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command as TokioCommand};

use crate::cli::CliBackend;

const CLEAR_ENV_VARS: &[&str] = &[
    "OPENCODE", "OPENCODE_PID", "OPENCODE_PROCESS_ROLE",
    "OPENCODE_HOME", "OPENCODE_RUN_ID",
];

pub struct InteractiveStream {
    child: Child,
    reader: BufReader<tokio::process::ChildStdout>,
}

impl InteractiveStream {
    pub fn spawn(
        backend: &dyn CliBackend,
        workdir: &str,
        prompt: &str,
        continue_session: bool,
    ) -> std::io::Result<Self> {
        let mut std_cmd = backend.build_command(workdir, prompt, continue_session);
        for var in CLEAR_ENV_VARS {
            std_cmd.env_remove(var);
        }
        std_cmd.stdout(Stdio::piped());
        std_cmd.stderr(Stdio::null());
        std_cmd.stdin(Stdio::null());
        let mut tokio_cmd: TokioCommand = std_cmd.into();
        let mut child = tokio_cmd.spawn()?;
        let stdout = child.stdout.take().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::Other, "failed to capture stdout")
        })?;
        Ok(Self {
            child,
            reader: BufReader::new(stdout),
        })
    }

    pub async fn read_line(&mut self) -> std::io::Result<Option<String>> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).await?;
        if n == 0 {
            return Ok(None);
        }
        let clean = strip_ansi(&line);
        Ok(Some(clean))
    }

    pub async fn kill(&mut self) {
        let _ = self.child.kill().await;
    }
}

fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(s, "").to_string()
}
