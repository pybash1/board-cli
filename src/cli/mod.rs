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
}