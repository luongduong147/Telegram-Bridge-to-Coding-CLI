use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum NdjsonEvent {
    #[serde(rename = "step_start")]
    StepStart,
    #[serde(rename = "step_finish")]
    StepFinish {
        #[serde(default)]
        part: Option<StepFinishPart>,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        #[serde(default)]
        part: Option<ToolPart>,
    },
    #[serde(rename = "text")]
    Text {
        #[serde(default)]
        part: Option<TextPart>,
    },
    #[serde(rename = "res_sys")]
    ResSys,
    #[serde(rename = "res_user")]
    ResUser,
    #[serde(rename = "res")]
    Res,
}

#[derive(Debug, Deserialize)]
pub struct StepFinishPart {
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub tokens: Option<TokenUsage>,
}

#[derive(Debug, Deserialize)]
pub struct ToolPart {
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub state: Option<ToolState>,
}

#[derive(Debug, Deserialize)]
pub struct ToolState {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TextPart {
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenUsage {
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub input: Option<u64>,
    #[serde(default)]
    pub output: Option<u64>,
    #[serde(default)]
    pub reasoning: Option<u64>,
    #[serde(default)]
    pub cache: Option<CacheUsage>,
}

#[derive(Debug, Deserialize)]
pub struct CacheUsage {
    #[serde(default)]
    pub write: Option<u64>,
    #[serde(default)]
    pub read: Option<u64>,
}

pub struct ParsedOutput {
    pub tool_calls: Vec<String>,
    pub text_content: Vec<String>,
    pub total_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
}

pub fn parse_ndjson(line: &str) -> Option<NdjsonEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() || !trimmed.starts_with('{') {
        return None;
    }
    serde_json::from_str::<NdjsonEvent>(trimmed).ok()
}

pub fn accumulate(events: &[NdjsonEvent]) -> ParsedOutput {
    let mut tool_calls = Vec::new();
    let mut text_content = Vec::new();
    let mut total_tokens = None;
    let mut reasoning_tokens = None;
    let mut cache_read = None;
    let mut cache_write = None;

    for event in events {
        match event {
            NdjsonEvent::ToolUse { part } => {
                if let Some(p) = part {
                    let tool_name = p.tool.as_deref().unwrap_or("unknown");
                    let title = p.state.as_ref().and_then(|s| s.title.as_deref()).unwrap_or("");
                    let output = p.state.as_ref().and_then(|s| s.output.as_deref()).unwrap_or("");
                    let input_desc = p.state.as_ref().and_then(|s| {
                        s.input.as_ref().map(|v| match v {
                            serde_json::Value::Object(m) => {
                                let keys: Vec<&str> = m.keys().map(|k| k.as_str()).collect();
                                keys.join(", ")
                            }
                            other => format!("{:?}", other),
                        })
                    }).unwrap_or_default();
                    let summary = format!("[{tool_name}] {title} | input: {input_desc} | output: {output}");
                    tool_calls.push(summary);
                }
            }
            NdjsonEvent::Text { part } => {
                if let Some(p) = part {
                    if let Some(t) = &p.text {
                        text_content.push(t.clone());
                    }
                }
            }
            NdjsonEvent::StepFinish { part } => {
                if let Some(p) = part {
                    if let Some(tokens) = &p.tokens {
                        total_tokens = tokens.total;
                        reasoning_tokens = tokens.reasoning;
                        if let Some(cache) = &tokens.cache {
                            cache_read = cache.read;
                            cache_write = cache.write;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    ParsedOutput {
        tool_calls,
        text_content,
        total_tokens,
        reasoning_tokens,
        cache_read,
        cache_write,
    }
}
