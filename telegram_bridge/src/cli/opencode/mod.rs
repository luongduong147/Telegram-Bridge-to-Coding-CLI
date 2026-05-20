use std::process::Command as StdCommand;

use crate::config::CliConfig;
use crate::ui::BlockType;

pub struct OpenCodeBackend {
    config: CliConfig,
}

impl OpenCodeBackend {
    pub fn new(config: &CliConfig) -> Self {
        Self { config: config.clone() }
    }

    fn build_run_command(&self, workdir: &str, prompt: &str) -> StdCommand {
        let mut cmd = StdCommand::new(&self.config.bin_path);
        cmd.arg("run");
        cmd.arg("--dir");
        cmd.arg(workdir);
        cmd.arg("--model");
        cmd.arg("opencode/deepseek-v4-flash-free");
        cmd.arg(prompt);
        cmd
    }

    fn build_json_command(&self, workdir: &str, prompt: &str) -> StdCommand {
        let mut cmd = StdCommand::new(&self.config.bin_path);
        cmd.arg("run");
        cmd.arg("--format");
        cmd.arg("json");
        cmd.arg("--dir");
        cmd.arg(workdir);
        cmd.arg("--model");
        cmd.arg("opencode/deepseek-v4-flash-free");
        cmd.arg(prompt);
        cmd
    }
}

impl super::CliBackend for OpenCodeBackend {
    fn build_command(
        &self,
        workdir: &str,
        prompt: &str,
        _continue_session: bool,
    ) -> StdCommand {
        self.build_run_command(workdir, prompt)
    }

    fn build_json_command(
        &self,
        workdir: &str,
        prompt: &str,
    ) -> StdCommand {
        self.build_json_command(workdir, prompt)
    }

    fn process_line(&mut self, line: &str) -> Option<(BlockType, String)> {
        let t = line.trim();
        if t.is_empty() {
            return None;
        }

        if t == "(no output)" {
            return Some((BlockType::CommandExec, t.to_string()));
        }

        if t.starts_with('>') {
            return None;
        }

        let exec_prefixes = ["✱", "→", "←", "$ "];
        let exec_prefixes2 = ["Wrote", "Read", "Write", "Ran"];
        for p in &exec_prefixes {
            if t.starts_with(p) {
                return Some((BlockType::CommandExec, t.to_string()));
            }
        }
        for p in &exec_prefixes2 {
            if t.starts_with(p) {
                return Some((BlockType::CommandExec, t.to_string()));
            }
        }

        Some((BlockType::Thinking, t.to_string()))
    }
}
