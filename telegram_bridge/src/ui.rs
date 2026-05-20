use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::markdownv2;

#[derive(Clone, Debug, PartialEq)]
pub enum BlockType {
    Thinking,
    CommandExec,
}

impl BlockType {
    pub fn label(&self) -> &'static str {
        match self {
            BlockType::Thinking => "Response",
            BlockType::CommandExec => "Command",
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
}

impl MessageUiState {
    pub fn new() -> Self {
        Self {
            blocks: vec![],
            has_finished: false,
            is_hidden: false,
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

    fn truncate(s: &str) -> String {
        let max = 3000;
        if s.len() > max {
            let mut t = s[..max].to_string();
            t.push_str("... (trimmed)");
            t
        } else {
            s.to_string()
        }
    }

    pub fn render_markdown(&self) -> String {
        if self.is_hidden {
            return "\u{1f648} *Da an*".to_string();
        }
        let mut out = String::new();
        for block in &self.blocks {
            match block.block_type {
                BlockType::Thinking => {
                    if block.is_expanded {
                        out.push_str("\u{1f9e0} *Response:*\n");
                        let escaped = markdownv2::escape(&Self::truncate(&block.content));
                        out.push_str(&escaped);
                        out.push('\n');
                    } else {
                        let preview = Self::first_line(&block.content);
                        out.push_str(&format!(
                            "\u{1f9e0} *Response*  `{}`\n\n",
                            preview
                        ));
                    }
                }
                BlockType::CommandExec => {
                    if block.is_expanded {
                        out.push_str("\u{1f4bb} *Command:*\n```\n");
                        out.push_str(&Self::truncate(&block.content));
                        out.push_str("```\n");
                    } else {
                        let preview = Self::first_line(&block.content);
                        out.push_str(&format!(
                            "\u{1f4bb} *Command*  `{}`\n\n",
                            preview
                        ));
                    }
                }
            }
        }
        if !self.has_finished {
            out.push_str("\n\u{23f3} *Dang xu ly\\.\\.\\.*");
        }
        out
    }

    pub fn build_keyboard(&self) -> InlineKeyboardMarkup {
        let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();
        if !self.is_hidden {
            for (i, block) in self.blocks.iter().enumerate() {
                let emoji = match block.block_type {
                    BlockType::Thinking => "\u{1f9e0}",
                    BlockType::CommandExec => "\u{1f4bb}",
                };
                let (action, label) = if block.is_expanded {
                    ("collapse", "\u{1f53d} Thu gon")
                } else {
                    ("expand", "\u{25b6}\u{fe0f} Xem")
                };
                rows.push(vec![InlineKeyboardButton::callback(
                    format!("{} {} {}", emoji, block.block_type.label(), label),
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
