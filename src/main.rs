use anyhow::Result;
use clap::Parser;

mod cli;
mod tui;
mod config;
mod error;

use cli::{Cli, Commands};
use tui::App;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Tui) => {
            // Launch TUI mode
            let mut app = App::new()?;
            app.run()?;
        }
        None => {
            // Show help when no command is specified
            println!("No command specified. Use 'board --help' to see available commands.");
            println!("Run 'board tui' to launch the TUI interface.");
        }
    }

    Ok(())
}
