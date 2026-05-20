use std::process::Command as StdCommand;

use crate::config::CliConfig;
use crate::ui::BlockType;

enum CodexPhase {
    Exec,
    Thinking,
}

pub struct CodexBackend {
    config: CliConfig,
    phase: CodexPhase,
}

impl CodexBackend {
    pub fn new(config: &CliConfig) -> Self {
        Self {
            config: config.clone(),
            phase: CodexPhase::Exec,
        }
    }
}

impl super::CliBackend for CodexBackend {
    fn build_command(
        &self,
        workdir: &str,
        prompt: &str,
        _continue_session: bool,
    ) -> StdCommand {
        let mut cmd = StdCommand::new(&self.config.bin_path);
        cmd.arg("exec");
        cmd.arg(prompt);
        cmd.current_dir(workdir);
        cmd
    }

    fn build_json_command(
        &self,
        workdir: &str,
        prompt: &str,
    ) -> StdCommand {
        let mut cmd = StdCommand::new(&self.config.bin_path);
        cmd.arg("exec");
        cmd.arg("--json");
        cmd.arg(prompt);
        cmd.current_dir(workdir);
        cmd
    }

    fn process_line(&mut self, line: &str) -> Option<(BlockType, String)> {
        let t = line.trim();
        if t.is_empty() {
            return None;
        }

        if t == "--------" {
            return None;
        }

        if let Some(rest) = t.strip_prefix('[') {
            if let Some(idx) = rest.find(']') {
                let content = rest[idx + 1..].trim();

                if content.starts_with("tokens used:")
                    || content.starts_with("User instructions:")
                {
                    return None;
                }

                if content == "codex" || content.starts_with("codex ") {
                    self.phase = CodexPhase::Thinking;
                    return None;
                }

                if content.is_empty() {
                    return None;
                }

                return Some((BlockType::CommandExec, t.to_string()));
            }
        }

        match self.phase {
            CodexPhase::Exec => Some((BlockType::CommandExec, t.to_string())),
            CodexPhase::Thinking => Some((BlockType::Thinking, t.to_string())),
        }
    }
}
