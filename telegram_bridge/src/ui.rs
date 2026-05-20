use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::markdownv2;

const TELEGRAM_MAX: usize = 4096;
const SPLIT_MARGIN: usize = 200;

#[derive(Clone, Debug, PartialEq)]
pub enum BlockType {
    UserContext,
    Thinking,
    CommandExec,
}

impl BlockType {
    pub fn label(&self) -> &'static str {
        match self {
            BlockType::UserContext => "Context",
            BlockType::Thinking => "Response",
            BlockType::CommandExec => "Command",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            BlockType::UserContext => "\u{1f4ac}",
            BlockType::Thinking => "\u{1f9e0}",
            BlockType::CommandExec => "\u{1f4bb}",
        }
    }
}

#[derive(Clone, Debug)]
pub struct UiBlock {
    pub block_type: BlockType,
    pub content: String,
    pub is_expanded: bool,
}

#[derive(Clone, Debug)]
pub struct MessageUiState {
    pub blocks: Vec<UiBlock>,
    pub has_finished: bool,
    pub is_hidden: bool,
    pub sent_message_ids: Vec<i32>,
}

impl MessageUiState {
    pub fn new() -> Self {
        Self {
            blocks: vec![],
            has_finished: false,
            is_hidden: false,
            sent_message_ids: vec![],
        }
    }

    pub fn push_line(&mut self, line: &str) {
        match self.blocks.last_mut() {
            Some(block) => {
                block.content.push_str(line);
                block.content.push('\n');
            }
            None => {
                self.blocks.push(UiBlock {
                    block_type: BlockType::Thinking,
                    content: line.to_string() + "\n",
                    is_expanded: false,
                });
            }
        }
    }

    pub fn start_new_block(&mut self, block_type: BlockType, line: &str) {
        self.blocks.push(UiBlock {
            block_type,
            content: line.to_string() + "\n",
            is_expanded: false,
        });
    }

    fn first_line(s: &str) -> String {
        markdownv2::escape(s.lines().next().unwrap_or(""))
    }

    fn short_summary(s: &str, max: usize) -> String {
        let first = s.lines().next().unwrap_or("");
        let escaped = markdownv2::escape(first);
        if escaped.len() > max {
            format!("{}...", &escaped[..max.saturating_sub(3)])
        } else {
            escaped
        }
    }

    pub fn render_markdown(&self) -> String {
        if self.is_hidden {
            return "\u{1f648} *Da an*".to_string();
        }
        let mut out = String::new();
        let mut first = true;
        for block in &self.blocks {
            if !first { out.push('\n'); }
            first = false;
            match block.block_type {
                BlockType::UserContext => {
                    out.push_str(&format!("{} *User Context*\n", BlockType::UserContext.emoji()));
                    for line in block.content.lines() {
                        out.push_str(&markdownv2::escape(line));
                        out.push('\n');
                    }
                }
                BlockType::Thinking => {
                    if block.is_expanded {
                        out.push_str(&format!("{} *Response:*\n", BlockType::Thinking.emoji()));
                        let escaped = markdownv2::escape(&block.content);
                        out.push_str(&escaped);
                        out.push('\n');
                    } else {
                        let preview = Self::first_line(&block.content);
                        out.push_str(&format!(
                            "{} *Response*  `{}`\n",
                            BlockType::Thinking.emoji(),
                            preview
                        ));
                    }
                }
                BlockType::CommandExec => {
                    if block.is_expanded {
                        out.push_str(&format!("{} *Command:*\n```\n", BlockType::CommandExec.emoji()));
                        out.push_str(&block.content);
                        out.push_str("```\n");
                    } else {
                        let preview = Self::short_summary(&block.content, 80);
                        out.push_str(&format!(
                            "{} *Command*  `{}`\n",
                            BlockType::CommandExec.emoji(),
                            preview
                        ));
                    }
                }
            }
        }
        if !self.has_finished {
            out.push_str(&format!("\n{} *Dang xu ly\\.\\.\\.*", "\u{23f3}"));
        }
        out
    }

    pub fn split_into_messages(&self) -> Vec<String> {
        let full = self.render_markdown();
        if full.len() <= TELEGRAM_MAX {
            return vec![full];
        }
        let mut parts = Vec::new();
        let mut start = 0;
        while start < full.len() {
            let end = (start + TELEGRAM_MAX - SPLIT_MARGIN).min(full.len());
            if end < full.len() {
                if let Some(break_pos) = full[start..end].rfind('\n') {
                    parts.push(full[start..start + break_pos].to_string());
                    start += break_pos + 1;
                } else {
                    parts.push(full[start..end].to_string());
                    start = end;
                }
            } else {
                parts.push(full[start..].to_string());
                break;
            }
        }
        parts
    }

    pub fn build_keyboard(&self) -> InlineKeyboardMarkup {
        let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();
        if !self.is_hidden {
            for (i, block) in self.blocks.iter().enumerate() {
                if block.block_type == BlockType::UserContext {
                    continue;
                }
                let (action, label) = if block.is_expanded {
                    ("collapse", "\u{1f53d} Thu gon")
                } else {
                    ("expand", "\u{25b6}\u{fe0f} Xem")
                };
                rows.push(vec![InlineKeyboardButton::callback(
                    format!("{} {} {}", block.block_type.emoji(), block.block_type.label(), label),
                    format!("{}:{}", action, i),
                )]);
            }
        }
        if !self.has_finished {
            rows.push(vec![InlineKeyboardButton::callback(
                "\u{23f9} Dung",
                "interrupt",
            )]);
        }
        let (hide_action, hide_label) = if self.is_hidden {
            ("unhide", "\u{1f441} Hien")
        } else {
            ("hide", "\u{1f648} An")
        };
        rows.push(vec![InlineKeyboardButton::callback(hide_label, hide_action)]);
        InlineKeyboardMarkup::new(rows)
    }
}
