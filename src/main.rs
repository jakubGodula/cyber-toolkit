//!
//! # Cyber Toolkit Manager
//! 
//! A command-line utility to manage collections of cybersecurity tools (roles) on Arch Linux-based systems.
//! It fetches tool lists from a predefined GitHub repository, installs/uninstalls them using `pacman`,
//! and manages a local configuration file (`~/.roles/roles.cnf`) to keep track of active roles.

// CLI-specific constants
const AUTHOR: &str = "Jakub Godula";
const VERSION: &str = "0.1.1";
const ABOUT: &str = "Manages roles and associated tools for Athena OS.";

use clap::Parser;
use std::env; // For env::args in main, if needed beyond what lib.rs provides for elevate_to_root

// Import all public items from our library
use cyber_toolkit::*;

/// Defines the command-line arguments accepted by the application.
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
    /// These files are expected to be located in the repository defined by `REPO_URL` (now in lib.rs).
    /// - If `--remove` is used, these are the roles to remove from the configuration and system.
    /// - Otherwise (default), these roles are added/synced.
    /// This argument is ignored if `--list-all` is used.
    role_files: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Centralized root check and elevation at the very beginning.
    // You might want to un-comment this block if root is required for most operations.
    /*
    if !check_if_user_is_root() { // check_if_user_is_root is now from the library
        elevate_to_root()?;      // elevate_to_root is now from the library
    }
    */
    
    let cli = Cli::parse(); // Parse command-line arguments
    
    
    // Dispatch logic based on parsed arguments
    if cli.list_all {
        display_available_roles_and_tools().await?; // From library
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
            println!("No specific command given. Listing available role names from the repository:");
            println!("Use '--list-all' to see roles and their tools.");
            println!("Use 'cyber-toolkit <role_name>...' to add/sync roles.");
            println!("Use 'cyber-toolkit --update <role_name>...' to set system to specified roles.");
            println!("Use 'cyber-toolkit --remove <role_name>...' to remove roles.");
            display_available_roles().await?; // From library
        }
    } else if cli.update {
        handle_update_command(&cli.role_files).await?; // From library
    } else if cli.remove {
        println!("Executing REMOVE command for roles: {:?}", cli.role_files);
        handle_remove_command(&cli.role_files).await?; // From library
    } else {
        // This is the ADD/SYNC case based on provided role files.
        // The check for cli.role_files.is_empty() earlier should ideally prevent reaching here if it's empty,
        // but if it can be reached, this is the add/sync action.
        if !cli.role_files.is_empty() {
            println!("Executing ADD/SYNC command for roles: {:?}", cli.role_files);
            handle_add_command(&cli.role_files).await?; // From library
        } else {
            // This else branch for add/sync with empty role_files should not be hit
            // if the logic above is correct. Consider revising overall if-else structure if needed.
            eprintln!("No role files provided for add/sync operation. This path should ideally not be reached.");
            println!("Use '--list-all' or provide role names.");
        }
    }
    
    //println!("\n--- Operation finished ---");
    Ok(())
}
