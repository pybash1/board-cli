use anyhow::Result;
use clap::Parser;
use std::io::{self, Read};

mod cli;
mod tui;
mod config;
mod error;
mod api;

use cli::{Cli, Commands, DeviceActions};
use tui::App;
use api::{BoardClient, BoardClientConfig, DeviceCode, PasteId};
use config::AppConfig;

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
            let mut config = AppConfig::load()?;
            let mut client = BoardClient::new()?;
            let device_code = client.register_device().await?;

            // Save the device code to config
            config.set_device_code(device_code.clone())?;

            println!("Device registered: {}", device_code);
            println!("Device code saved to config file at: {}", AppConfig::config_path()?.display());
        }
        Commands::Device { action } => {
            let mut config = AppConfig::load()?;

            match action {
                DeviceActions::Show => {
                    if let Some(device_code) = config.get_device_code() {
                        println!("Current device code: {}", device_code);
                        println!("Config file: {}", AppConfig::config_path()?.display());
                    } else {
                        println!("No device code configured");
                        println!("Use 'board device new' to register a new device");
                    }
                }
                DeviceActions::Set { code } => {
                    let device_code = DeviceCode::from(code.clone());
                    config.set_device_code(device_code)?;
                    println!("Device code set: {}", code);
                    println!("Config saved to: {}", AppConfig::config_path()?.display());
                }
                DeviceActions::Clear => {
                    config.clear_device_code()?;
                    println!("Device code cleared from config");
                    println!("Config saved to: {}", AppConfig::config_path()?.display());
                }
                DeviceActions::New => {
                    let mut client = BoardClient::new()?;
                    let device_code = client.register_device().await?;
                    config.set_device_code(device_code.clone())?;
                    println!("New device registered: {}", device_code);
                    println!("Device code saved to config file at: {}", AppConfig::config_path()?.display());
                }
            }
        }
        Commands::Tui => {
            // This should not happen as TUI is handled separately
            unreachable!()
        }
    }
    Ok(())
}

async fn get_api_client() -> Result<BoardClient> {
    let mut config = AppConfig::load()?;

    // Device code is always required - get it from config or register new device
    let device_code = if let Some(config_device_code) = config.get_device_code() {
        config_device_code
    } else if let Ok(env_device_code) = std::env::var("BOARD_DEVICE_CODE") {
        // Migrate from environment variable to config
        let device_code = DeviceCode::from(env_device_code);
        config.set_device_code(device_code.clone())?;
        eprintln!("Migrated device code from environment variable to config file");
        device_code
    } else {
        // Register new device and save to config
        eprintln!("No device code configured. Registering new device...");
        let mut client = BoardClient::new()?;
        let device_code = client.register_device().await?;
        config.set_device_code(device_code.clone())?;
        eprintln!("Device registered: {}", device_code);
        eprintln!("Device code saved to config file at: {}", AppConfig::config_path()?.display());
        device_code
    };

    // Create client config with both device code (required) and app password (optional)
    let mut client_config = BoardClientConfig::from_app_config(&config);
    client_config.device_code = Some(device_code);

    Ok(BoardClient::with_config(client_config)?)
}