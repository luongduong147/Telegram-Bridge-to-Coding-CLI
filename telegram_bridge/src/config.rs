use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct CliConfig {
    pub name: String,
    pub bin_path: String,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub bot_token: String,
    pub authorized_chat_id: Option<i64>,
    pub workdirs: Vec<PathBuf>,
    pub default_workdir_index: usize,
    pub clis: Vec<CliConfig>,
    pub default_cli: String,
}

fn parse_cli_env(index: usize) -> Option<CliConfig> {
    let name = std::env::var(format!("CLI_{}_NAME", index)).ok()?;
    let bin_path = std::env::var(format!("CLI_{}_BIN", index)).ok()?;
    Some(CliConfig { name, bin_path })
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bot_token = std::env::var("BOT_TOKEN")
            .or_else(|_| std::env::var("TELEGRAM_BOT_TOKEN"))
            .map_err(|_| anyhow::anyhow!("BOT_TOKEN or TELEGRAM_BOT_TOKEN is required"))?;

        let authorized_chat_id = match std::env::var("AUTHORIZED_CHAT_ID") {
            Ok(v) => Some(v.parse().map_err(|_| {
                anyhow::anyhow!("AUTHORIZED_CHAT_ID must be a numeric user ID")
            })?),
            Err(_) => None,
        };

        let workdirs: Vec<PathBuf> = match std::env::var("WORKDIRS") {
            Ok(v) => v.split(',').map(|s| PathBuf::from(s.trim())).collect(),
            Err(_) => {
                let single = std::env::var("OPENCODE_WORKDIR")
                    .unwrap_or_else(|_| "/workspace".to_string());
                vec![PathBuf::from(single)]
            }
        };

        let default_workdir_index = std::env::var("DEFAULT_WORKDIR_INDEX")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|&i| i < workdirs.len())
            .unwrap_or(0);

        let mut clis: Vec<CliConfig> = (1..=32).filter_map(parse_cli_env).collect();

        if clis.is_empty() {
            let legacy_bin = std::env::var("OPENCODE_BIN")
                .unwrap_or_else(|_| "opencode".to_string());
            clis.push(CliConfig {
                name: "opencode".to_string(),
                bin_path: legacy_bin,
            });
        }

        let default_cli = std::env::var("DEFAULT_CLI")
            .unwrap_or_else(|_| "opencode".to_string());

        Ok(Self {
            bot_token,
            authorized_chat_id,
            workdirs,
            default_workdir_index,
            clis,
            default_cli,
        })
    }

    pub fn is_authorized(&self, chat_id: i64) -> bool {
        match self.authorized_chat_id {
            Some(id) => chat_id == id,
            None => true,
        }
    }

    pub fn current_workdir(&self) -> &PathBuf {
        &self.workdirs[self.default_workdir_index]
    }

    pub fn get_cli(&self, name: &str) -> Option<&CliConfig> {
        self.clis.iter().find(|c| c.name == name)
    }

    pub fn default_cli_config(&self) -> &CliConfig {
        self.get_cli(&self.default_cli)
            .unwrap_or_else(|| &self.clis[0])
    }
}
