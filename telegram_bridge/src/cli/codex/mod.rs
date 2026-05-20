use std::process::Command as StdCommand;

use crate::config::CliConfig;
use crate::ui::BlockType;

enum CodexPhase {
    UserContext,
    Exec,
    Thinking,
}

pub struct CodexBackend {
    config: CliConfig,
    phase: CodexPhase,
    ctx_lines: Vec<String>,
    user_context_emitted: bool,
}

impl CodexBackend {
    pub fn new(config: &CliConfig) -> Self {
        Self {
            config: config.clone(),
            phase: CodexPhase::UserContext,
            ctx_lines: Vec::new(),
            user_context_emitted: false,
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

    fn get_user_context(&mut self) -> Option<String> {
        if self.user_context_emitted || self.ctx_lines.is_empty() {
            return None;
        }
        self.user_context_emitted = true;
        Some(format!("---\n{}\n---", self.ctx_lines.join("\n")))
    }

    fn process_line(&mut self, line: &str) -> Option<(BlockType, String)> {
        let t = line.trim();
        if t.is_empty() {
            return None;
        }

        match self.phase {
            CodexPhase::UserContext => {
                if t == "--------" {
                    self.phase = CodexPhase::Exec;
                    return None;
                }
                self.ctx_lines.push(t.to_string());
                None
            }
            CodexPhase::Exec => {
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
                Some((BlockType::CommandExec, t.to_string()))
            }
            CodexPhase::Thinking => Some((BlockType::Thinking, t.to_string())),
        }
    }
}
