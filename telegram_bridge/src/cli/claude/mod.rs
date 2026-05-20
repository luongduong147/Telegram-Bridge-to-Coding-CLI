use std::process::Command as StdCommand;

use crate::config::CliConfig;
use crate::ui::BlockType;

pub struct ClaudeBackend {
    config: CliConfig,
}

impl ClaudeBackend {
    pub fn new(config: &CliConfig) -> Self {
        Self { config: config.clone() }
    }
}

impl super::CliBackend for ClaudeBackend {
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
        Some((BlockType::CommandExec, t.to_string()))
    }
}
