use std::process::Command as StdCommand;

use crate::config::CliConfig;
use crate::ui::BlockType;

pub trait CliBackend: Send + Sync {
    fn build_command(
        &self,
        workdir: &str,
        prompt: &str,
        continue_session: bool,
    ) -> StdCommand;
    fn process_line(&mut self, line: &str) -> Option<(BlockType, String)>;
}

pub mod opencode;
mod codex;
mod claude;

pub fn create_backend(config: &CliConfig) -> Box<dyn CliBackend> {
    match config.name.as_str() {
        "opencode" => Box::new(opencode::OpenCodeBackend::new(config)),
        "codex" => Box::new(codex::CodexBackend::new(config)),
        "claude" => Box::new(claude::ClaudeBackend::new(config)),
        _ => Box::new(opencode::OpenCodeBackend::new(config)),
    }
}
