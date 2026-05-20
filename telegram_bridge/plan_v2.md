# Telegram Bridge — Plan v2

> Kế thừa từ PLAN.md (v1), tập trung vào **4 tính năng shipping tiếp theo**.
> Mục tiêu: biến bot từ "gửi prompt → chờ → nhận kết quả" thành phiên bản real-time, đa CLI, có điều khiển.


---

## I. Tóm tắt trạng thái hiện tại (v1)

### Đã hoạt động (v1)
- Bot nhận message Telegram, dispatch slash command hoặc plain text
- Plain text → spawn CLI subprocess (async, pipe stdout)
- Stream output line-by-line qua `BufReader<ChildStdout>`
- Debounce 1.5s + rate-limit 1s, edit message Telegram real-time
- UI blocks: Thinking / CommandExec với expand/collapse
- Multi-CLI: OpenCode, Codex, Claude Code (config riêng)
- Interrupt: AtomicBool flag, kill subprocess
- Filter output nhạy cảm (token, key)
- Session 10 phút timeout, auth single chat_id
- Callback query handler (expand/collapse/interrupt)

### Hạn chế còn lại
| Vấn đề | Chi tiết |
|--------|----------|
| **Batch mode → buffered output** | `opencode run` dùng pipe → stdout bị block-buffered, streaming không thực sự real-time |
| **Không interactive** | Không thể gửi lệnh `/compact` vào subprocess đang chạy |
| **Idle timeout cứng** | Hết 10 phút → kill ngay, không cơ hội compact/save conversation |
| **Không persistent session** | Mỗi lần prompt là session mới, không tận dụng context của CLI |

### Giữ nguyên từ v1
- Cấu trúc project (modular)
- Cơ chế auth
- Cơ chế session timeout
- Cách dispatch slash command

---

## II. Feature 1: Đa CLI (Codex, Claude Code, OpenCode)

> **Cập nhật v2.1:** `run_args` không còn được dùng trực tiếp làm args cho CLI.
> Thay vào đó, mỗi CLI có `spawn_args` (args để khởi động interactive session)
> và prompt được ghi vào stdin sau khi spawn.

### 2.1 Cấu hình

```rust
// src/config.rs
#[derive(Clone, Debug)]
pub struct CliConfig {
    pub name: String,          // "opencode", "codex", "claude"
    pub bin_path: String,      // Đường dẫn tuyệt đối hoặc tên binary
    pub spawn_args: Vec<String>, // Args spawn interactive session: ["--dir"] hoặc []
    pub prompt_prefix: String,  // Prefix thêm vào prompt nếu cần (vd: "")
    pub active: bool,          // User có muốn dùng CLI này không
}
```

### 2.2 Biến môi trường

> **Cập nhật v2.1:** `CLI_N_ARGS` đổi thành `CLI_N_SPAWN_ARGS` (args để spawn
> interactive session, không phải args run). Prompt được ghi vào stdin, không
> phải arg dòng lệnh.

```env
# CLI configurations (có thể khai báo nhiều)
CLI_1_NAME=opencode
CLI_1_BIN=/var/account/duonglnt/npm_global/bin/opencode
CLI_1_SPAWN_ARGS=--dir

CLI_2_NAME=codex
CLI_2_BIN=/usr/local/bin/codex
CLI_2_SPAWN_ARGS=

CLI_3_NAME=claude
CLI_3_BIN=/usr/local/bin/claude
CLI_3_SPAWN_ARGS=

# CLI mặc định khi user không chọn
DEFAULT_CLI=opencode
```

### 2.3 Cơ chế chọn CLI

- User gửi `/cli` → Bot reply inline keyboard với danh sách CLI đã cấu hình
- User bấm nút → Bot set `active_cli` cho session
- Khi prompt được gửi, executor dùng CLI config tương ứng

```
User: /cli
Bot: [InlineKeyboard]
     [🔧 OpenCode] [🤖 Codex] [🟣 Claude]
     ─────────────────────────────
     Hiện tại: OpenCode
```

### 2.4 Executor đa CLI (interactive mode)

```rust
// src/executor.rs
pub fn spawn_interactive(cli_config: &CliConfig, workdir: &str) -> Result<Box<dyn InteractiveProcess>> {
    // Mỗi CLI tự quyết định cách spawn interactive session
    let backend = create_backend(cli_config);
    backend.spawn_interactive(workdir)
}

/// Trait cho interactive process — ghi prompt, đọc output, gửi slash command
pub trait InteractiveProcess: Send {
    fn name(&self) -> &str;
    /// Gửi prompt vào stdin
    fn send_prompt(&mut self, prompt: &str) -> io::Result<()>;
    /// Gửi slash command (vd: /compact)
    fn send_slash(&mut self, cmd: &str) -> io::Result<()>;
    /// Đọc dòng output tiếp theo (blocking read từ PTY master)
    fn read_line(&mut self) -> io::Result<Option<String>>;
    /// Kiểm tra nếu output chứa prompt (CLI đã sẵn sàng nhận input mới)
    fn detects_prompt(&self, line: &str) -> bool;
    /// Kill process
    fn kill(&mut self) -> io::Result<()>;
    /// Đợi process kết thúc
    fn wait(&mut self) -> io::Result<i32>;
}

// Ví dụ cho OpenCode: spawn `opencode --dir <workdir>`, ghi prompt vào stdin
impl InteractiveProcess for OpenCodeProcess {
    fn send_prompt(&mut self, prompt: &str) -> io::Result<()> {
        use std::io::Write;
        writeln!(self.stdin, "{}", prompt)
    }
    fn send_slash(&mut self, cmd: &str) -> io::Result<()> {
        use std::io::Write;
        writeln!(self.stdin, "/{}", cmd)
    }
    fn detects_prompt(&self, line: &str) -> bool {
        // OpenCode interactive hiển thị ">" hoặc "opencode>" làm prompt
        line.trim_end().ends_with('>') || line.trim() == "opencode>"
    }
}
```
  
> **Chi tiết implement:** `InteractiveProcess` được implement khác nhau cho mỗi CLI:
> - **OpenCode:** spawn `opencode --dir <wd>`, prompt = `>`, slash = `/compact`
> - **Codex:** spawn `codex --dir <wd>`, prompt = `►`, slash = `/compact`
> - **Claude:** spawn `claude`, prompt = `>`, slash = `/compact`

### 2.5 Marker patterns cho từng CLI

Mỗi CLI có pattern riêng để phân biệt thinking vs command-exec, và để phát hiện prompt:

| CLI | Thinking markers | Command markers | Prompt marker |
|-----|-----------------|-----------------|---------------|
| OpenCode | `[Think]`, dòng bắt đầu bằng `# `, nội dung phân tích | `Running:`, `➔`, dòng bắt đầu bằng `$ ` | `> ` ở cuối dòng |
| Codex | `<thinking>`, `Thinking...`, văn bản giải thích | ``` ` ``` (code block), `Executing:` | `► ` ở cuối dòng |
| Claude | Non-ansi text (mặc định), dòng phân tích | `$ `, command trong code block | `> ` ở cuối dòng |

> Prompt marker dùng để phát hiện CLI đã hoàn thành xử lý prompt hiện tại và sẵn sàng nhận input mới.

---

## III. Feature 2: Stream Thinking + UI Blocks

> **Cập nhật v2.1:** Chuyển từ pipe → PTY, từ batch → interactive.
> Thay đổi chính:
> 1. Spawn CLI interactive mode qua PTY (unbuffered)
> 2. Ghi prompt vào stdin sau khi spawn
> 3. Đọc output real-time từ PTY master
> 4. Phát hiện prompt marker để biết khi nào xử lý xong
> 5. Idle timeout: gửi `/compact` → save log → kill

### 3.1 Kiến trúc streaming

```
User gửi prompt
    ↓
Bot spawn CLI interactive (opencode --dir <wd>) qua PTY
    ↓
Đợi prompt ">" → CLI sẵn sàng
    ↓
Ghi prompt vào PTY stdin
    ↓
Stream reader (đọc từng dòng từ PTY master) — UNBUFFERED
    ↓
Phân loại dòng → Thinking / CommandExec / Prompt
    ↓
Debounce 1.5-2s → edit_message_text + inline keyboard
    ↓
Khi phát hiện prompt ">" → prompt hiện tại đã xử lý xong
    ↓
Nếu còn prompt trong queue → ghi prompt tiếp theo
    ↓
Nếu idle > 10 phút → gửi "/compact" → save log → kill session
```

### 3.2 Interactive Stream Reader (PTY-based)

```rust
// src/stream.rs — dùng portable-pty crate
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use portable_pty::{PtySize, native_pty_system, ChildKiller};

pub struct InteractiveStream {
    pty_reader: BufReader<Box<dyn std::io::Read + Send>>,
    pty_writer: Box<dyn std::io::Write + Send>,
    killer: Box<dyn ChildKiller + Send>,
    child_waiter: Option<Box<dyn std::thread::JoinHandle<std::io::Result<i32>>>>,
    prompt_pattern: String,
    is_running: Arc<AtomicBool>,
}

impl InteractiveStream {
    pub fn spawn(backend: &dyn CliBackend, workdir: &str) -> io::Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize::default())?;
        
        let mut cmd = std::process::Command::new(backend.bin_path());
        cmd.args(backend.spawn_args());
        cmd.arg(workdir);
        cmd.env("CLICOLOR", "0");        // Tắt màu
        cmd.env("TERM", "xterm-256color"); // Giả lập terminal
        
        let child = pair.slave.spawn_command(cmd)?;
        let reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let killer = child.take_killer()?;
        
        Ok(Self {
            pty_reader: BufReader::new(reader),
            pty_writer: writer,
            killer,
            child_waiter: None,
            prompt_pattern: backend.prompt_pattern().to_string(),
            is_running: Arc::new(AtomicBool::new(true)),
        })
    }
    
    /// Ghi prompt vào stdin của CLI (blocking, dùng spawn_blocking)
    pub fn send_prompt(&mut self, prompt: &str) -> io::Result<()> {
        writeln!(self.pty_writer, "{}", prompt)
    }
    
    /// Gửi slash command
    pub fn send_slash(&mut self, cmd: &str) -> io::Result<()> {
        writeln!(self.pty_writer, "/{}", cmd)
    }
    
    /// Đọc dòng tiếp theo từ PTY master (unbuffered, real-time)
    pub fn read_line(&mut self) -> io::Result<Option<String>> {
        let mut line = String::new();
        let n = self.pty_reader.read_line(&mut line)?;
        if n == 0 { return Ok(None); }
        // Strip ANSI escape sequences
        let clean = strip_ansi(&line);
        Ok(Some(clean))
    }
    
    /// Kiểm tra dòng có phải prompt không
    pub fn is_prompt(&self, line: &str) -> bool {
        line.trim().ends_with(&self.prompt_pattern)
    }
    
    pub fn kill(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        let _ = self.killer.kill();
    }
}

/// Strip ANSI escape sequences khỏi output
fn strip_ansi(s: &str) -> String {
    // Regex loại bỏ \x1b[...m, \x1b[...K, \x1b[?...h, ...
    let re = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(s, "").to_string()
}
```

> **Lưu ý:** `portable-pty` là thư viện cross-platform quản lý PTY lifecycle.
> Trên Linux dùng `forkpty()` + fd操控, trên macOS dùng `forkpty()` tương tự.
> Thay thế: dùng `nix::pty::openpty` + `std::os::unix::io::FromRawFd` (Unix-only).

### 3.3 Data structures cho UI

```rust
// src/ui.rs (module mới)
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum BlockType {
    Thinking,
    CommandExec,
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
    pub last_update: Option<std::time::Instant>,
    pub has_finished: bool,
}

impl MessageUiState {
    pub fn new() -> Self { /* ... */ }

    /// Phân loại dòng mới nhận được từ stream
    /// Bỏ qua dòng prompt (đã được detect bởi InteractiveStream)
    pub fn classify_line(cli_name: &str, line: &str) -> Option<BlockType> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        match cli_name {
            "opencode" => {
                if trimmed.starts_with("Running:") || trimmed.starts_with("\u{2794}") || trimmed.starts_with("$ ") {
                    Some(BlockType::CommandExec)
                } else if trimmed.starts_with("[Think]") || trimmed.starts_with("# ") {
                    Some(BlockType::Thinking)
                } else {
                    None // Tiếp tục block hiện tại
                }
            }
            "codex" => {
                if trimmed == "<thinking>" || trimmed == "Thinking..." || trimmed.starts_with("I think") {
                    Some(BlockType::Thinking)
                } else if trimmed.starts_with("Executing:") || trimmed.contains("```") {
                    Some(BlockType::CommandExec)
                } else {
                    None
                }
            }
            "claude" => {
                if trimmed.starts_with("$ ") || trimmed.starts_with("```") {
                    Some(BlockType::CommandExec)
                } else if trimmed.starts_with("I'll") || trimmed.starts_with("Let me") {
                    Some(BlockType::Thinking)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Thêm dòng vào block hiện tại
    pub fn push_line(&mut self, cli_name: &str, line: &str) {
        let last = self.blocks.last_mut();
        if let Some(block) = last {
            // Kiểm tra xem có cần chuyển block type không
            if let Some(new_type) = Self::classify_line(cli_name, line) {
                if new_type != block.block_type {
                    // Lưu block cũ, mở block mới
                    self.blocks.push(UiBlock {
                        block_type: new_type,
                        content: line.to_string() + "\n",
                        is_expanded: false,
                    });
                    return;
                }
            }
            block.content.push_str(line);
            block.content.push('\n');
        } else {
            // Block đầu tiên — mặc định Thinking
            self.blocks.push(UiBlock {
                block_type: BlockType::Thinking,
                content: line.to_string() + "\n",
                is_expanded: false,
            });
        }
    }

    /// Render thành Markdown để gửi lên Telegram
    pub fn render_markdown(&self) -> String {
        let mut output = String::new();
        for block in &self.blocks {
            match block.block_type {
                BlockType::Thinking => {
                    if block.is_expanded {
                        output.push_str("🧠 *Thinking Process:*\n");
                        output.push_str(&format!("```text\n{}\n```\n", Self::truncate_content(&block.content)));
                    } else {
                        let line_count = block.content.lines().count();
                        let preview = Self::first_line(&block.content);
                        output.push_str(&format!(
                            "🧠 *Thinking...* ({} dòng ẩn) `{}`\n\n",
                            line_count, preview
                        ));
                    }
                }
                BlockType::CommandExec => {
                    if block.is_expanded {
                        output.push_str("💻 *Command Execution:*\n");
                        output.push_str(&format!("```bash\n{}\n```\n", block.content));
                    } else {
                        let line_count = block.content.lines().count();
                        let preview = Self::first_line(&block.content);
                        output.push_str(&format!(
                            "💻 *Executing...* ({} dòng ẩn) `{}`\n\n",
                            line_count, preview
                        ));
                    }
                }
            }
        }
        if !self.has_finished {
            output.push_str("\n⏳ *Đang xử lý...*");
        }
        output
    }

    /// Tạo inline keyboard
    pub fn build_keyboard(&self) -> Vec<Vec<InlineKeyboardButton>> {
        let mut rows = Vec::new();
        for (i, block) in self.blocks.iter().enumerate() {
            let emoji = match block.block_type {
                BlockType::Thinking => "🧠",
                BlockType::CommandExec => "💻",
            };
            let action = if block.is_expanded { "🔽 Thu gọn" } else { "▶️ Xem chi tiết" };
            let cb_data = if block.is_expanded {
                format!("collapse:{}", i)
            } else {
                format!("expand:{}", i)
            };
            rows.push(vec![InlineKeyboardButton::callback(
                format!("{} {} {}", emoji, block.block_type.label(), action),
                cb_data,
            )]);
        }
        if !self.has_finished {
            rows.push(vec![InlineKeyboardButton::callback(
                "⏹ Dừng",
                "interrupt",
            )]);
        }
        rows
    }
}
```

### 3.4 Prompt handler (interactive mode + idle timeout)

```rust
// src/handler/prompt.rs — interactive mode, PTY streaming, idle timeout
use crate::ui::{MessageUiState, BlockType};
use tokio::time::{sleep, Duration};
use crate::executor::{self, is_interrupted};
use crate::stream::InteractiveStream;
use crate::cli::create_backend;

const STREAM_DEBOUNCE: Duration = Duration::from_millis(1500);
const TELEGRAM_RATE_LIMIT: Duration = Duration::from_millis(1000);
const IDLE_TIMEOUT: Duration = Duration::from_secs(600); // 10 phút
const SLASH_COMPACT: &str = "compact";

pub async fn handle_prompt_interactive(
    bot: &Bot,
    msg: &Message,
    text: &str,
    config: &Config,
    session: &mut OpenCodeSession,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;

    // 1. Gửi message tạm để edit về sau
    let sent = bot.send_message(chat_id, "\u{1f680} <b>Dang khoi tao...</b>")
        .parse_mode(ParseMode::Html)
        .await?;
    let message_id = sent.id;

    // 2. Lấy CLI backend + spawn interactive session
    let cli_config = config.get_active_cli();
    let backend = create_backend(cli_config);
    let workdir = config.current_workdir().to_string_lossy().to_string();
    let mut stream = InteractiveStream::spawn(backend.as_ref(), &workdir)?;

    // 2b. Đợi prompt ">" xuất hiện → CLI đã sẵn sàng
    wait_for_prompt(&mut stream, &backend.name()).await?;

    // 2c. Ghi prompt vào stdin
    stream.send_prompt(text)?;

    // 3. Streaming loop + idle timeout
    let mut ui_state = MessageUiState::new();
    let mut last_edit = Instant::now();
    let mut has_pending = false;
    let mut last_activity = Instant::now();

    loop {
        // Kiểm tra interrupt
        if is_interrupted() {
            stream.send_slash(SLASH_COMPACT).ok();
            sleep(Duration::from_secs(2)).await;
            stream.kill();
            break;
        }

        // Kiểm tra idle timeout
        let idle = last_activity.elapsed();
        if idle >= IDLE_TIMEOUT {
            // Gửi /compact để thu gọn conversation
            stream.send_slash(SLASH_COMPACT).ok();
            sleep(Duration::from_secs(5)).await; // Chờ compact hoàn tất
            
            // Đọc nốt output sau compact
            while let Ok(Some(_)) = stream.read_line() {}
            
            // Kill session
            stream.kill();
            ui_state.has_finished = true;
            let msg_text = format!("{}\n\n\u{23f3} <b>Het thoi gian cho (10 phut). Da compact va dong session.</b>",
                ui_state.render_html());
            bot.edit_message_text(chat_id, message_id, msg_text)
                .parse_mode(ParseMode::Html)
                .reply_markup(InlineKeyboardMarkup::default())
                .await?;
            return Ok(());
        }

        tokio::select! {
            line = async { stream.read_line().ok().flatten() } => {
                match line {
                    Some(line) => {
                        // Bỏ qua dòng prompt
                        if stream.is_prompt(&line) {
                            continue;
                        }
                        last_activity = Instant::now();
                        
                        let filtered = filter_sensitive(&line);
                        if let Some(bt) = backend.classify_line(&filtered) {
                            let should_start_new = ui_state.blocks.last()
                                .map(|b| b.block_type != bt)
                                .unwrap_or(true);
                            if should_start_new {
                                ui_state.start_new_block(bt, &filtered);
                            } else {
                                ui_state.push_line(&filtered);
                            }
                        } else {
                            ui_state.push_line(&filtered);
                        }
                        has_pending = true;
                    }
                    None => break, // EOF = process đã kết thúc
                }
            }
            _ = sleep(Duration::from_millis(100)) => {
                if has_pending && last_edit.elapsed() >= STREAM_DEBOUNCE {
                    let md = ui_state.render_html();
                    let kb = ui_state.build_keyboard();
                    let wait = TELEGRAPH_RATE_LIMIT.saturating_sub(last_edit.elapsed());
                    if !wait.is_zero() {
                        sleep(wait).await;
                    }
                    bot.edit_message_text(chat_id, message_id, &md)
                        .parse_mode(ParseMode::Html)
                        .reply_markup(kb)
                        .await.ok();
                    last_edit = Instant::now();
                    has_pending = false;
                }
            }
        }
    }

    // 4. Hoàn thành
    ui_state.has_finished = true;
    let final_text = ui_state.render_html();
    let kb = ui_state.build_keyboard();
    bot.edit_message_text(chat_id, message_id, format!("{}\n\n\u{2705} <b>Hoan thanh</b>", final_text))
        .parse_mode(ParseMode::Html)
        .reply_markup(kb)
        .await?;

    session.touch();
    Ok(())
}

/// Đợi cho đến khi CLI hiển thị prompt hoặc timeout
async fn wait_for_prompt(stream: &mut InteractiveStream, cli_name: &str) -> anyhow::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if let Ok(Some(line)) = stream.read_line() {
            tracing::debug!("[{} init] {}", cli_name, line.trim());
            if stream.is_prompt(&line) {
                return Ok(());
            }
        } else {
            sleep(Duration::from_millis(100)).await;
        }
    }
    Err(anyhow::anyhow!("Timeout waiting for CLI prompt"))
}
```

> **Thay đổi chính so với v1:**
> 1. `StreamReader::spawn()` → `InteractiveStream::spawn()` (PTY, interactive)
> 2. Prompt ghi vào stdin sau khi spawn, không phải arg
> 3. Thêm `wait_for_prompt()` — đợi CLI sẵn sàng trước khi gửi prompt
> 4. Idle timeout: gửi `/compact` → đợi → kill, thay vì kill ngay
> 5. Dòng prompt bị skip khỏi UI state

### 3.5 Xử lý callback query (expand/collapse)

```rust
// Trong bot.rs hoặc handler mới: callback_handler.rs
pub async fn handle_callback(
    bot: &Bot,
    q: CallbackQuery,
    // Cần lưu MessageUiState ở đâu đó — session extension hoặc global state
) -> anyhow::Result<()> {
    let (action, index_str) = q.data.split_once(':').unwrap_or(("", "0"));
    let index: usize = index_str.parse().unwrap_or(0);

    // Tìm MessageUiState tương ứng với message này
    let mut ui_state = get_ui_state(q.message?.id).await?;

    if action == "expand" {
        if let Some(block) = ui_state.blocks.get_mut(index) {
            block.is_expanded = true;
        }
    } else if action == "collapse" {
        if let Some(block) = ui_state.blocks.get_mut(index) {
            block.is_expanded = false;
        }
    } else if action == "interrupt" {
        // Feature 3
        interrupt_current_session().await;
        ui_state.has_finished = true;
        let text = format!("{}\n\n⏹ *Đã dừng theo yêu cầu*", ui_state.render_markdown());
        bot.edit_message_text(q.message?.chat.id, q.message?.id, text)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(InlineKeyboardMarkup::default())
            .await?;
        return Ok(());
    }

    let text = ui_state.render_markdown();
    let keyboard = ui_state.build_keyboard();
    bot.edit_message_text(q.message?.chat.id, q.message?.id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;

    bot.answer_callback_query(q.id).await?;
    Ok(())
}
```

### 3.6 Gom chunk & chống rate limit

| Chiến lược | Mô tả |
|------------|-------|
| **Debounce 1.5s** | Chỉ gửi update sau 1.5s kể từ lần cuối, gom toàn bộ dòng trong khoảng thời gian đó |
| **Rate limit 1s** | Không edit message quá 1 lần/giây (Telegram: 1 msg/chat/s) |
| **Max 20 msg/phút** | Giới hạn mềm, nếu vượt quá thì bỏ qua lỗi (`.ok()`) |
| **Batch lines** | Gom tối đa 50 dòng mỗi lần update để tránh quá tải render |
| **Telegram message size** | Nếu output > 4096 bytes, cắt bớt (dùng split_message) |

### 3.7 Ví dụ luồng streaming hoàn chỉnh

```
Time  User              Bot                              Subprocess
0s    [Gửi prompt]      🚀 Đang khởi tạo...              opencode run ...
1s                       [Debounce 1.5s]
2.5s                     🧠 Thinking... (5 dòng ẩn) [▶️]  Đang phân tích
4s                       🧠 Thinking... (12 dòng ẩn) [▶️] Phân tích xong
                         💻 Executing... (3 dòng ẩn) [▶️] Running: cargo test
5.5s                     🧠 [▶️] 💻 [▶️]                   Đang chạy test
8s                       ⏹ [Dừng]                         Finished!
                         ✅ Hoàn thành (exit code 0)
                         🧠 [🔽] 💻 [🔽]
```

---

## IV. Feature 3: Interrupt

### 4.1 Cơ chế

Khi đang stream, user có thể:
- Bấm nút **⏹ Dừng** trên inline keyboard
- Gửi lệnh `/interrupt` hoặc `/stop`

### 4.2 Global state cho process đang chạy

```rust
// src/executor.rs — global current process tracker
use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;

pub static INTERRUPT_FLAG: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

pub fn set_interrupt() {
    INTERRUPT_FLAG.store(true, Ordering::SeqCst);
}

pub fn clear_interrupt() {
    INTERRUPT_FLAG.store(false, Ordering::SeqCst);
}

pub fn is_interrupted() -> bool {
    INTERRUPT_FLAG.load(Ordering::SeqCst)
}

// Trong StreamReader
pub async fn read_line(&mut self) -> Option<String> {
    if is_interrupted() {
        self.kill();
        return None;
    }
    let mut line = String::new();
    self.reader.read_line(&mut line).await.ok()?;
    if line.is_empty() { None } else { Some(line) }
}
```

### 4.3 Slash command `/interrupt`

```rust
// Thêm vào BotCommand
#[command(description = "Stop current execution")]
Interrupt,
```

Xử lý: set flag → process tự dừng ở lần đọc dòng tiếp theo → stream kết thúc sạch sẽ.

### 4.4 Callback query `interrupt`

Khi user bấm nút Dừng, xử lý trong callback handler:
1. Set `INTERRUPT_FLAG`
2. Đợi stream reader tự kết thúc (tối đa 2s timeout)
3. Force kill nếu cần
4. Gửi message "⏹ Đã dừng theo yêu cầu"

---

## V. Feature 4: Workdir List

### 5.1 Cấu hình

```env
# workdirs (danh sách cách nhau bằng dấu phẩy)
WORKDIRS=/var/account/duonglnt/project1,/var/account/duonglnt/project2,/var/account/duonglnt/a2a_consoc/agent_2_agent

# Index của workdir mặc định (0-based)
DEFAULT_WORKDIR_INDEX=0
```

### 5.2 Config struct

```rust
pub struct Config {
    // ... các field cũ
    pub workdirs: Vec<PathBuf>,
    pub default_workdir_index: usize,
}
```

### 5.3 Slash command `/workdir`

```
User: /workdir
Bot: [InlineKeyboard]
     [1. 📁 project1] [2. 📁 project2] [3. 📁 agent_2_agent]
     ─────────────────────────────────
     Hiện tại: project1

User chọn project2
Bot: ✅ Đã chuyển sang project2
```

### 5.4 Logic xử lý

- Session lưu `current_workdir_index`
- Khi prompt được gửi, dùng `workdirs[current_workdir_index]`
- `/status` hiển thị workdir đang dùng + tất cả workdir có sẵn

---

## VI. Cập nhật cấu trúc thư mục

Mỗi CLI được đóng gói trong một folder riêng, chứa toàn bộ logic riêng (cách spawn, marker patterns, args). Module `cli/mod.rs` đóng vai trò factory trait + dispatcher.

```
telegram_bridge/
├── Cargo.toml
├── .env.example
└── src/
    ├── main.rs                 # Entry point
    ├── config.rs               # Config tổng: bot, workdir list
    ├── bot.rs                  # Bot setup + callback query handler
    ├── session.rs              # Session: active_cli, active_workdir
    ├── filter.rs               # [MỚI] Lọc output nhạy cảm
    ├── ui.rs                   # [MỚI] MessageUiState, UiBlock, BlockType
    ├── cli/                    # [MỚI] CLI abstraction
    │   ├── mod.rs              # CliBackend trait + factory
    │   ├── opencode/           # OpenCode implementation
    │   │   ├── mod.rs          # CliBackend impl cho OpenCode
    │   │   ├── config.rs       # Config riêng của OpenCode (bin path, args)
    │   │   ├── executor.rs     # Spawn opencode run
    │   │   └── markers.rs      # Pattern: [Think], Running:, ➔, $ ...
    │   ├── codex/              # Codex implementation
    │   │   ├── mod.rs          # CliBackend impl cho Codex
    │   │   ├── config.rs       # Config riêng của Codex
    │   │   ├── executor.rs     # Spawn codex
    │   │   └── markers.rs      # Pattern: <thinking>, Executing:, ``` ...
    │   └── claude/             # Claude implementation
    │       ├── mod.rs          # CliBackend impl cho Claude
    │       ├── config.rs       # Config riêng của Claude
    │       ├── executor.rs     # Spawn claude
    │       └── markers.rs      # Pattern: $, ```, I'll, Let me ...
    ├── handler/
    │   ├── mod.rs              # Dispatch mở rộng + callback routing
    │   ├── slash.rs            # Slash commands (+ /cli, /workdir, /interrupt)
    │   ├── prompt.rs           # Streaming prompt handler
    │   └── callback.rs         # [MỚI] Xử lý callback query
```

### 6.1 `cli/mod.rs` — Trait + Factory (cập nhật cho interactive mode)

```rust
pub trait CliBackend: Send + Sync {
    fn name(&self) -> &str;
    fn bin_path(&self) -> &str;
    fn spawn_args(&self) -> &[String];
    /// Pattern để detect prompt (vd: ">", "►")
    fn prompt_pattern(&self) -> &str;
    fn classify_line(&self, line: &str) -> Option<BlockType>;
}

pub fn create_backend(config: &CliConfig) -> Box<dyn CliBackend> {
    match config.name.as_str() {
        "opencode" => Box::new(opencode::OpenCodeBackend::new(config)),
        "codex"    => Box::new(codex::CodexBackend::new(config)),
        "claude"   => Box::new(claude::ClaudeBackend::new(config)),
        _ => Box::new(opencode::OpenCodeBackend::new(config)),
    }
}
```

### 6.2 Ví dụ: `cli/opencode/mod.rs` (interactive mode)

```rust
pub struct OpenCodeBackend {
    config: CliConfig,
}

impl CliBackend for OpenCodeBackend {
    fn name(&self) -> &str { "opencode" }
    fn bin_path(&self) -> &str { &self.config.bin_path }
    fn spawn_args(&self) -> &[String] { &self.config.spawn_args }
    fn prompt_pattern(&self) -> &str { ">" }
    
    fn classify_line(&self, line: &str) -> Option<BlockType> {
        let t = line.trim();
        if t.starts_with("Running:") || t.starts_with("\u{2794}") || t.starts_with("$ ") {
            Some(BlockType::CommandExec)
        } else if t.starts_with("[Think]") || t.starts_with("# ") {
            Some(BlockType::Thinking)
        } else {
            None
        }
    }
}
```

### 6.3 Ví dụ: `cli/codex/markers.rs`

```rust
pub fn classify_line(line: &str) -> Option<BlockType> {
    let trimmed = line.trim();
    if trimmed == "<thinking>" || trimmed == "Thinking..." || trimmed.starts_with("I think") {
        Some(BlockType::Thinking)
    } else if trimmed.starts_with("Executing:") || trimmed.contains("```") || trimmed.starts_with("Result:") {
        Some(BlockType::CommandExec)
    } else {
        None
    }
}
```

---

## VII. Security Review

### 7.1 Lộ token/key trong output

| Rủi ro | Biện pháp |
|--------|-----------|
| CLI output chứa BOT_TOKEN, API key, JWT | Filter regex: `[A-Za-z0-9_-]{20,}` thay bằng `***` |
| Output chứa đường dẫn nhạy cảm | Không vấn đề (user đã có quyền truy cập) |
| Env key bị leak | Thêm filter pattern: `(sk-[a-zA-Z0-9]+|ghp_[a-zA-Z0-9]+)` |

```rust
// src/filter.rs — module lọc output
const SENSITIVE_PATTERNS: &[&str] = &[
    r"(?i)(?:bot_token|api_key|secret|password)\s*[=:]\s*\S+",
    r"(sk-[a-zA-Z0-9]{20,})",    // OpenAI key pattern
    r"(ghp_[a-zA-Z0-9]{36})",    // GitHub PAT
    r"(gho_[a-zA-Z0-9]{36})",    // GitHub OAuth
    r"[A-Za-z0-9_-]{40,}",       // Generic long token
];

pub fn filter_sensitive(input: &str) -> String {
    let mut output = input.to_string();
    for pattern in SENSITIVE_PATTERNS {
        let re = Regex::new(pattern).unwrap();
        output = re.replace_all(&output, "***$1***").to_string();
    }
    output
}
```

### 7.2 Shell injection

| Rủi ro | Biện pháp |
|--------|-----------|
| Prompt chứa `; rm -rf /` | Dùng `Command::arg()` — Rust std::process::Command tự escape |
| Prompt chứa ký tự đặc biệt | `Command::arg()` an toàn, không qua shell |
| Codex/Claude args injection | Validate args, không cho user tuỳ biến args tuỳ ý |

### 7.3 Auth & data isolation

| Vấn đề | Giải pháp |
|---------|-----------|
| Multi-user workdir conflict | Mỗi user 1 session riêng (nếu cần multi-user sau này) |
| Interrupt giữa user khác | Interrupt flag chỉ ảnh hưởng process của chính user đó (dùng session_id) |
| Token trong backup/log | Log không ghi BOT_TOKEN. Dùng `tracing` với env-filter, không ghi raw |

### 7.4 Checklist

- [x] BOT_TOKEN không bao giờ được log
- [x] AUTHORIZED_CHAT_ID không bao giờ được log
- [x] Command::arg() — không có shell injection
- [x] Filter output trước khi gửi lên Telegram
- [x] Timeout 10p — process tự động kill
- [x] AtomicBool interrupt — không race condition
- [ ] Callback query data không chứa thông tin nhạy cảm

---

## VIII. Test scenarios

### Scenario 1: Stream thinking mới, 2 blocks

```
Input: "hãy refactor hàm calculate()"

Expected output (sau khi stream kết thúc):
  🧠 Thinking Process:
  ```text
  Đang phân tích hàm calculate()...
  Phát hiện vấn đề: thiếu error handling...
  ```
  💻 Command Execution:
  ```bash
  cat src/math.rs
  ```
  ✅ Hoàn thành (exit code 0)
  [🧠 🔽 Thu gọn] [💻 🔽 Thu gọn]
```

### Scenario 2: User bấm "Xem chi tiết" → block mở rộng

```
Callback data: expand:0
Expected:
  🧠 [🔽 Thu gọn]
  > Đang phân tích hàm calculate()...
  > Phát hiện vấn đề: thiếu error handling...
  💻 Executing... (3 dòng ẩn) `cat src/math.rs`
```

### Scenario 3: User bấm "Thu gọn" → block ẩn

```
Callback data: collapse:0
Expected:
  🧠 Thinking... (2 dòng ẩn) `Đang phân tích hàm`
  💻 Executing... (3 dòng ẩn) `cat src/math.rs`
```

### Scenario 4: Interrupt giữa chừng

```
User bấm ⏹ Dừng
Expected:
  🧠 Thinking... (5 dòng ẩn)
  ⏹ Đã dừng theo yêu cầu
  [🧠 ▶️ Xem chi tiết]
  (Không còn nút Dừng)
```

### Scenario 5: Multi-CLI — chuyển đổi Codex → OpenCode

```
Step 1: User gửi /cli
        Bot hiện: [🔧 OpenCode] [🤖 Codex] [🟣 Claude]
        Hiện tại: OpenCode

Step 2: User bấm 🤖 Codex
        Bot: ✅ Đã chuyển sang Codex

Step 3: User gửi "viết function fibonacci"
        Bot stream output từ Codex (pattern markers khác)
```

### Scenario 6: Đổi workdir

```
Step 1: User gửi /workdir
        Bot: [1. 📁 project1] [2. 📁 project2]
        Hiện tại: project1

Step 2: User bấm project2
        Bot: ✅ Đã chuyển sang /var/account/duonglnt/project2
```

### Scenario 7: Rate limit — stream quá nhanh

```
Input: prompt ngắn, CLI trả về rất nhanh (< 1s)
Expected:
  - Bot chỉ gửi 1 lần edit (sau 1.5s debounce)
  - Không gửi nhiều lần trong 1 giây
  - Kết quả cuối cùng đầy đủ
```

### Scenario 8: Output chứa API key

```
Input: prompt có chứa token sk-xxxx
Expected:
  - Token bị filter: ***sk-xxxx***
  - Không có secret nào rò rỉ ra Telegram
  - Bot log không chứa token raw
```

### Scenario 9: Timeout

```
Input: prompt khiến CLI chạy > 10 phút
Expected:
  - Bot tự động kill process
  - Gửi message: "⏳ Quá thời gian chờ (10 phút)"
  - Session vẫn còn, user có thể gửi prompt mới
```

### Scenario 10: Interrupt callback timeout

```
User bấm Dừng, nhưng process không chịu dừng (SIGKILL needed)
Expected:
  - Bot set flag → đợi 2s
  - Nếu process còn sống → force kill (child.kill())
  - Gửi message: "⏹ Đã dừng (force kill)"
```

---

## IX. Kế hoạch triển khai (Implementation order)

### Phase 1 — Nền tảng (đã hoàn thành)
1. ✅ Refactor `Config`: thêm `workdirs: Vec<PathBuf>` (Feature 4)
2. ✅ Thêm `src/ui.rs`: `BlockType`, `UiBlock`, `MessageUiState`, render + keyboard
3. ✅ Thêm `src/stream.rs`: async `StreamReader` (pipe-based)
4. ✅ Refactor `executor.rs`: spawn không blocking, hỗ trợ `INTERRUPT_FLAG`
5. ✅ Multi-CLI: `cli/mod.rs` trait + OpenCode / Codex / Claude backends
6. ✅ `handler/slash.rs`: `/cli`, `/workdir`, `/interrupt`
7. ✅ `handler/callback.rs`: expand/collapse/interrupt
8. ✅ `src/filter.rs`: lọc token/key

### Phase 2 — Interactive Mode via PTY (đang làm)
9. **Cập nhật `StreamReader` → `InteractiveStream`** (PTY-based):
   - Dùng `portable-pty` crate thay vì pipe
   - Ghi prompt vào stdin sau spawn (không phải arg)
   - Phát hiện prompt marker để biết CLI sẵn sàng
   - Strip ANSI escape sequences khỏi output
10. **Cập nhật `CliBackend` trait**:
    - Bỏ `build_command()`, thêm `bin_path()`, `spawn_args()`, `prompt_pattern()`
    - Mỗi backend implement `prompt_pattern()` riêng (`>`, `►`, `>`)
11. **Cập nhật `handler/prompt.rs`**:
    - Interactive spawn → đợi prompt → ghi prompt → stream
    - Idle timeout: `/compact` → save → kill (thay vì kill ngay)
12. **Thêm `wait_for_prompt()` utility**: đợi CLI sẵn sàng với timeout 30s

### Phase 3 — Persistent Session (mới)
13. Lưu conversation log khi idle timeout hoặc interrupt
14. Resume session với flag continue (nếu CLI hỗ trợ)
15. Tối ưu: tái sử dụng interactive session cho nhiều prompt liên tiếp

### Phase 4 — Hoàn thiện
16. Update `.env.example` (CLI_N_ARGS → CLI_N_SPAWN_ARGS)
17. Cập nhật `DEVELOPER.md`
18. Kiểm tra toàn bộ test scenarios
19. Cleanup: xoá `StreamReader` pipe-based cũ

---

## X. Dependencies mới (Cargo.toml)

```toml
[dependencies]
# Hiện tại — giữ nguyên
teloxide = { version = "0.13", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
dotenvy = "0.15"
chrono = "0.4"
thiserror = "2"
anyhow = "1"
reqwest = "0.11"
regex = "1"                  # Filter sensitive output + strip ANSI

# Mới
portable-pty = "0.8"         # Cross-platform PTY cho interactive CLI mode
```

---

## XI. Rủi ro & Mitigation

| Rủi ro | Xác suất | Ảnh hưởng | Mitigation |
|--------|----------|------------|------------|
| Telegram rate limit (429) | Cao | Mất update | Retry với backoff, debounce 1.5s |
| Process treo, không kill được | Thấp | Treo session | 2s timeout → SIGKILL |
| Multi-user gây race condition | Trung bình | Gửi nhầm output | AtomicBool + per-session tracking |
| Output quá lớn (>4KB) | Cao | Trim content | Split message, dùng pagination |
| Regex filter false positive | Thấp | User mất data | Whitelist pattern, log warning |
| PTY không available (Windows) | Thấp | Không chạy được | portable-pty fallback |
| ANSI strip mất data | Trung bình | Output thiếu nội dung | Kiểm tra kỹ regex, test với output thật |
| Prompt detection sai | Trung bình | Stream không kết thúc | Timeout cứng 10p fallback |
| `/compact` mất context | Thấp | CLImất memory | Có thể dùng `/compact` hoặc save log |
| PTY buffer overflow | Thấp | Mất output | Đọc thường xuyên, buffer lớn |
