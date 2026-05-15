# Telegram Bridge — Hướng dẫn vận hành

## 1. Tổng quan repo

Telegram Bridge là bot Rust kết nối Telegram với OpenCode CLI.
Kiến trúc: Telegram message → teloxide bot → OpenCode CLI → trả kết quả về Telegram.

```
telegram_bridge/
├── Cargo.toml          # Thông tin package + dependencies
├── Cargo.lock
├── Dockerfile          # Build Docker multi-stage
├── .env                # Biến môi trường (BOT_TOKEN, ...)
├── .env.example        # Mẫu env
├── src/
│   ├── main.rs         # Entry point: init logger, config, start bot
│   ├── config.rs       # Đọc env vars: BOT_TOKEN, AUTHORIZED_CHAT_ID, ...
│   ├── bot.rs          # Khởi tạo teloxide bot, dispatcher
│   ├── session.rs      # Quản lý phiên làm việc (timeout 10 phút)
│   ├── executor.rs     # Chạy opencode CLI, đợi kết quả
│   └── handler/
│       ├── mod.rs      # Dispatch message: slash command vs prompt
│       ├── slash.rs    # /start, /check, /status, /help
│       └── prompt.rs   # Gửi prompt tới opencode + trả về
├── bridge.log          # File log khi chạy (nếu có)
└── PLAN.md             # Tài liệu thiết kế
```

**Luồng xử lý:**
1. User gửi tin nhắn text → bot nhận qua teloxide long-polling
2. Kiểm tra authorize (AUTHORIZED_CHAT_ID)
3. Nếu là slash command → xử lý riêng
4. Nếu là text thường → chạy `opencode run --dir <workdir> "<prompt>"` và trả kết quả

---

## 2. Build

### Yêu cầu
- Rust 1.84+ (rustup)
- Telegram Bot Token (từ @BotFather)

### Build local
```bash
cd /var/account/duonglnt/telegram_bridge

# Build release
cargo build --release

# Binary tại: target/release/telegram_bridge
```

### Build Docker
```bash
docker build -t telegram-bridge .
```

---

## 3. Chạy

### Chuẩn bị env
```bash
cp .env.example .env
# Điền các biến:
#   TELEGRAM_BOT_TOKEN=<token từ BotFather>
#   AUTHORIZED_CHAT_ID=<user_id Telegram>
#   OPENCODE_WORKDIR=<đường dẫn workspace>
#   OPENCODE_BIN=<đường dẫn opencode binary>
```

### Chạy local
```bash
# Trực tiếp
cargo run --release

# Hoặc dùng binary
./target/release/telegram_bridge
```

### Chạy Docker
```bash
docker run -d \
  --name telegram-bridge \
  --restart unless-stopped \
  -e TELEGRAM_BOT_TOKEN="your_token" \
  -e AUTHORIZED_CHAT_ID="your_user_id" \
  -e OPENCODE_WORKDIR="/workspace" \
  -e OPENCODE_BIN="opencode" \
  -v /var/account/duonglnt:/workspace \
  telegram-bridge
```

---

## 4. Kiểm tra trạng thái (Check status)

### Qua Telegram slash commands
- `/check` — Kiểm tra session còn hiệu lực hay hết hạn (10 phút)
- `/status` — Trạng thái chi tiết: workdir, session active, thời gian còn lại
- `/help` — Danh sách tất cả commands

### Kiểm tra process
```bash
# Bot có đang chạy không
ps aux | grep telegram_bridge

# Docker
docker ps | grep telegram-bridge
docker logs telegram-bridge
```

### Lưu ý
Bot hiện tại chạy OpenCode CLI trực tiếp qua `std::process::Command` (không dùng tmux như bản thiết kế cũ).

---

## 5. Kill (Dừng bot)

### Local
```bash
# Tìm PID
pgrep -f telegram_bridge

# Kill
kill <PID>
# Hoặc
pkill -f telegram_bridge
```

### Docker
```bash
docker stop telegram-bridge
docker rm telegram-bridge
```



---

## 6. Kiểm tra log

### Local (stdout/stderr)
Mặc định log ra stdout với định dạng tracing-subscriber.
Có thể redirect khi chạy:
```bash
./target/release/telegram_bridge 2>&1 | tee -a bridge.log
```

### Docker
```bash
# Live log
docker logs -f telegram-bridge

# Log gần đây (100 dòng cuối)
docker logs --tail 100 telegram-bridge

# Log kèm timestamp
docker logs -t telegram-bridge
```

### Cấu hình log level
```bash
# Chạy với log level DEBUG để debug chi tiết
RUST_LOG=telegram_bridge=debug ./target/release/telegram_bridge

# Hoặc trace
RUST_LOG=trace ./target/release/telegram_bridge
```

### File log
Nếu đã redirect ra file `bridge.log`:
```bash
tail -f bridge.log        # Live log
tail -n 100 bridge.log    # 100 dòng cuối
less bridge.log           # Xem toàn bộ
```

---

## 7. Các biến môi trường

| Biến | Mô tả | Bắt buộc |
|---|---|---|
| `TELEGRAM_BOT_TOKEN` | Token từ @BotFather | Có |
| `BOT_TOKEN` | Alias cho TELEGRAM_BOT_TOKEN | Có (1 trong 2) |
| `AUTHORIZED_CHAT_ID` | User ID được phép dùng bot | Không (public nếu để trống) |
| `OPENCODE_WORKDIR` | Working directory cho OpenCode | Không (mặc định: /workspace) |
| `OPENCODE_BIN` | Đường dẫn opencode binary | Không (mặc định: opencode) |

---

## 8. Slash commands trên Telegram

| Command | Chức năng |
|---|---|
| `/start` | Giới thiệu bot |
| `/check` | Kiểm tra session còn hiệu lực không |
| `/status` | Trạng thái chi tiết (workdir, thời gian còn lại) |
| `/help` | Danh sách commands |

Gửi text thường → bot gửi làm prompt cho OpenCode và trả kết quả.
