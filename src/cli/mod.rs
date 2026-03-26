use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "board")]
#[command(about = "A CLI/TUI application for managing boards")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Launch the TUI interface
    Tui,
    /// Create a new paste from stdin
    Create {
        /// Optional title for the paste
        #[arg(short, long)]
        title: Option<String>,
    },
    /// Get paste content by ID
    Get {
        /// The paste ID to retrieve
        paste_id: String,
    },
    /// List all paste IDs for your device
    List,
    /// Show all pastes with content
    Show,
    /// Register a new device
    Register,
    /// Device management commands
    Device {
        #[command(subcommand)]
        action: DeviceActions,
    },
}

#[derive(Subcommand)]
pub enum DeviceActions {
    /// Show current device code
    Show,
    /// Set device code manually
    Set {
        /// Device code to use
        code: String,
    },
    /// Clear the stored device code
    Clear,
    /// Register a new device and set it as current
    New,
}