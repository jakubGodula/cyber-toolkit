//! # Cyber Toolkit Manager
//! 
//! A command-line utility to manage collections of cybersecurity tools (roles) on Arch Linux-based systems.
//! It fetches tool lists from a predefined GitHub repository, installs/uninstalls them using `pacman`,
//! and manages a local configuration file (`~/.roles/roles.cnf`) to keep track of active roles.
//! 
//! ## Author
//! 
//! Jakub Godula
//! 
//! ## License
//! 
//! This project is licensed under the MIT License - see the LICENSE file for details.
//! 
//! ## Features
//! 
//! - Fetch and manage tool collections (roles) from a central repository
//! - Install/update tools using pacman
//! - Remove roles and their unique tools
//! - List available roles and their tools
//! - Maintain a local configuration of active roles
//! - Automatic privilege elevation when needed

// CLI-specific constants
const AUTHOR: &str = "Jakub Godula";
const VERSION: &str = "0.1.1";
const ABOUT: &str = "Manages roles and associated tools for Athena OS.";

use clap::Parser;
use std::env;

// Import all public items from our library
use cyber_toolkit::*;

/// Defines the command-line arguments accepted by the application.
/// 
/// This struct uses the `clap` derive macro to automatically generate
/// command-line argument parsing and help documentation.
/// 
/// # Fields
/// 
/// * `list_all` - Flag to list all available roles and their tools
/// * `remove` - Flag to indicate removal of roles and their unique tools
/// * `update` - Flag to install the desired toolset
/// * `current` - Flag to list the current state of the system
/// * `role_files` - Names of the role files to process
#[derive(Parser, Debug)]
#[clap(author = AUTHOR, version = VERSION, about = ABOUT, long_about = "Manages roles and associated tools for Athena OS. Use --list-all to see available roles and their tools. Provide role names to add/sync them. Use --remove with role names to remove them.")]
struct Cli {
    /// Flag to list all available roles from the repository and their tools.
    #[clap(long)]
    list_all: bool,

    /// Flag to indicate removal of roles and their unique tools.
    /// If present, the listed `role_files` will be removed.
    #[clap(short, long)]
    remove: bool,

    /// Flag to install the desired toolset
    /// If present, the listed `role_files` will be used to determin the desired state of the system.
    #[clap(short, long)]
    update: bool,

    /// Flag to list the current state of the system.
    #[clap(short, long)]
    current: bool,

    /// Names of the role files to process (e.g., blue-teamer.txt).
    /// These files are expected to be located in the repository defined by `REPO_URL`.
    /// - If `--remove` is used, these are the roles to remove from the configuration and system.
    /// - Otherwise (default), these roles are added/synced.
    /// This argument is ignored if `--list-all` is used.
    role_files: Vec<String>,
}

/// The main entry point of the application.
/// 
/// This function:
/// 1. Parses command-line arguments using the `Cli` struct
/// 2. Handles privilege elevation if needed
/// 3. Dispatches to the appropriate command handler based on the arguments
/// 
/// # Command Flow
/// 
/// * `--list-all`: Displays all available roles and their tools
/// * `--remove`: Removes specified roles and their unique tools
/// * `--update`: Updates the system to match the specified roles
/// * `--current`: Shows currently configured roles
/// * No flags: Adds/syncs specified roles
/// 
/// # Returns
/// 
/// * `Ok(())` - If the command was executed successfully
/// * `Err(Box<dyn Error>)` - If there was an error during execution
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Dispatch logic based on parsed arguments
    if cli.list_all {
        display_available_roles_and_tools().await?;
    } else if cli.role_files.is_empty() {
        if cli.update {
            eprintln!("Error: The -u/--update flag requires at least one role file name to be specified.");
            eprintln!("Usage: {} --update <ROLE_FILE_NAMES...>", env::args().next().unwrap_or_else(|| "cyber-toolkit".to_string()));
            std::process::exit(1);
        } else if cli.current {
            let cli_current = read_roles_from_config_file();
            println!("Current roles: {:?}", cli_current);
        } else if cli.remove {
            eprintln!("Error: The -r/--remove flag requires at least one role file name to be specified.");
            eprintln!("Usage: {} --remove <ROLE_FILE_NAMES...>", env::args().next().unwrap_or_else(|| "cyber-toolkit".to_string()));
            std::process::exit(1);
        } else {
            display_available_roles().await?;
        }
    } else if cli.update {
        handle_update_command(&cli.role_files).await?;
    } else if cli.remove {
        println!("Executing REMOVE command for roles: {:?}", cli.role_files);
        handle_remove_command(&cli.role_files).await?;
    } else {
        if !cli.role_files.is_empty() {
            println!("Executing ADD/SYNC command for roles: {:?}", cli.role_files);
            handle_add_command(&cli.role_files).await?;
        }
        display_available_roles().await?;
    }
    
    Ok(())
}
