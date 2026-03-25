# Board CLI

A Rust-based command line interface with TUI capabilities.

## Features

- Interactive TUI interface
- Configurable settings
- Error handling

## Usage

```bash
# Show help
board --help

# Launch TUI
board tui
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