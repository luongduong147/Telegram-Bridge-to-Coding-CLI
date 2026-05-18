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
}

impl super::CliBackend for OpenCodeBackend {
    fn build_command(
        &self,
        workdir: &str,
        prompt: &str,
        _continue_session: bool,
    ) -> StdCommand {
        let mut cmd = StdCommand::new(&self.config.bin_path);
        cmd.arg("run");
        cmd.arg("--dir");
        cmd.arg(workdir);
        cmd.arg("--model");
        cmd.arg("opencode/deepseek-v4-flash-free");
        cmd.arg(prompt);
        cmd
    }

    fn process_line(&mut self, line: &str) -> Option<(BlockType, String)> {
        let t = line.trim();
        if t.is_empty() {
            return None;
        }

        if t.starts_with('\u{2731}')
            || t.starts_with('\u{2192}')
            || t.starts_with('\u{2190}')
            || t.starts_with("$ ")
            || t.starts_with("Wrote")
            || t.starts_with("Read")
            || t.starts_with("Write")
            || t.starts_with("Ran")
            || t == "(no output)"
        {
            Some((BlockType::CommandExec, t.to_string()))
        } else if t.starts_with('>') {
            None
        } else {
            Some((BlockType::Thinking, t.to_string()))
        }
    }
}
