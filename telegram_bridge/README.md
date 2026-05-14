# Telegram Bridge — OpenCode

Rust bot nhận message từ Telegram, gửi làm prompt cho OpenCode CLI qua tmux session, và trả kết quả về Telegram.

## Yêu cầu

- Rust 1.84+
- tmux
- Telegram Bot Token (từ [@BotFather](https://t.me/botfather))

## Cài đặt & chạy

### Local

```bash
cp .env.example .env
# Sửa .env: điền BOT_TOKEN và AUTHORIZED_CHAT_ID

cargo run --release
```

### Docker

```bash
docker build -t telegram-bridge .

docker run -d \
  --name telegram-bridge \
  -e BOT_TOKEN="your_token" \
  -e AUTHORIZED_CHAT_ID="your_user_id" \
  -v /path/to/workspace:/workspace \
  telegram-bridge
```

## Slash Commands

| Command | Chức năng |
|---|---|
| `/start` | Giới thiệu bot |
| `/check` | Kiểm tra tmux session |
| `/status` | Trạng thái chi tiết + last output |
| `/kill` | Kill tmux session |
| `/restart` | Tạo lại tmux session |
| `/clear` | Clear tmux màn hình |
| `/exec <cmd>` | Chạy lệnh shell (debug) |
| `/help` | Danh sách commands |

Gửi tin nhắn text thường để gửi prompt tới OpenCode.
