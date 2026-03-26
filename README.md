# Board CLI

A Rust-based command line interface with TUI capabilities for managing Board API pastes.

## Features

- Interactive TUI interface
- Configurable settings with TOML configuration
- Multiple authentication methods (App Password and Device Code)
- Create, read, and list pastes
- Error handling
- Configuration management

## Authentication

Board CLI uses two authentication headers for API requests:

1. **Device Code (Required)**: Every API request requires a device code
2. **App Password (Optional)**: Additional authentication when needed

### Device Code Authentication

Device codes are always required. Register a new device:

```bash
board register
# or
board device new
```

Manage device codes:

```bash
# Show current device code
board device show

# Set device code manually
board device set your_device_code_here

# Clear device code
board device clear
```

### App Password Authentication

Set an app password in your configuration file at `~/.config/board/config.toml`:

```toml
app_password = "your_app_password_here"
```

If no app password is configured, an empty value will be sent.

### API Headers

All API requests include both headers:
- `Device-Code: your_device_code`
- `App-Password: your_app_password_or_empty`

## Usage

```bash
# Show help
board --help

# Launch TUI
board tui

# Create a paste from stdin
echo "Hello, World!" | board create

# Get paste content by ID
board get paste_id

# List all paste IDs
board list

# Show all pastes with content
board show

# Manage device code
board device new
board device show
board device set device_code
board device clear
```

## Configuration

Configuration is stored in `~/.config/board/config.toml`:

```toml
data_dir = "~/.board-cli"
theme = "default"
auto_save = true

# Device code (required) - set automatically via CLI commands
device_code = "your_device_code_here"

# App password (optional) - set manually in config
app_password = "your_app_password_here"
```

### TUI Controls

- **q** or **Esc**: Quit

## Installation

```bash
cargo build --release
```

## Development

```bash
# Show help (default when no command given)
cargo run

# Launch TUI
cargo run -- tui

# Run tests
cargo test
```

## Project Structure

```
src/
├── main.rs          # Entry point
├── cli/             # CLI argument parsing
├── tui/             # TUI interface
├── config/          # Configuration management
└── error/           # Error types
```