use anyhow::Result;
use clap::Parser;
use std::io::{self, Read};

mod cli;
mod tui;
mod config;
mod error;
mod api;

use cli::{Cli, Commands};
use tui::App;
use api::{BoardClient, DeviceCode, PasteId};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Tui) => {
            // Launch TUI mode (non-async)
            let mut app = App::new()?;
            app.run()?;
        }
        Some(cmd) => {
            // Handle async CLI commands
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(handle_async_command(cmd))?;
        }
        None => {
            // Show help when no command is specified
            println!("No command specified. Use 'board --help' to see available commands.");
            println!("Run 'board tui' to launch the TUI interface.");
        }
    }

    Ok(())
}

async fn handle_async_command(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Create { title: _ } => {
            // Read content from stdin
            let mut content = String::new();
            io::stdin().read_to_string(&mut content)?;

            let client = get_api_client().await?;
            let paste = client.create_paste(&content).await?;
            println!("{}", paste.url);
        }
        Commands::Get { paste_id } => {
            let client = get_api_client().await?;
            let content = client.get_paste(&PasteId::from(paste_id)).await?;
            print!("{}", content);
        }
        Commands::List => {
            let client = get_api_client().await?;
            let paste_ids = client.list_pastes().await?;
            for paste_id in paste_ids {
                println!("{}", paste_id);
            }
        }
        Commands::Show => {
            let client = get_api_client().await?;
            let pastes = client.get_all_pastes().await?;
            for paste in pastes {
                println!("=== {} ===", paste.id);
                println!("{}", paste.content);
                println!();
            }
        }
        Commands::Register => {
            let mut client = BoardClient::new()?;
            let device_code = client.register_device().await?;
            println!("Device registered: {}", device_code);
            println!("Set BOARD_DEVICE_CODE={} to reuse this device", device_code);
        }
        Commands::Tui => {
            // This should not happen as TUI is handled separately
            unreachable!()
        }
    }
    Ok(())
}

async fn get_api_client() -> Result<BoardClient> {
    let mut client = BoardClient::new()?;

    // Try to get device code from environment
    if let Ok(device_code) = std::env::var("BOARD_DEVICE_CODE") {
        client.set_device_code(DeviceCode::from(device_code));
    } else {
        // Register new device
        eprintln!("No device code found. Registering new device...");
        let device_code = client.register_device().await?;
        eprintln!("Device registered: {}", device_code);
        eprintln!("Set BOARD_DEVICE_CODE={} to reuse this device", device_code);
    }

    Ok(client)
}