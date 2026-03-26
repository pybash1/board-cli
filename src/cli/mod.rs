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
}