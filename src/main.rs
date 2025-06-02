//!
//! # Cyber Toolkit Manager
//! 
//! A command-line utility to manage collections of cybersecurity tools (roles) on Arch Linux-based systems.
//! It fetches tool lists from a predefined GitHub repository, installs/uninstalls them using `pacman`,
//! and manages a local configuration file (`~/.roles/roles.cnf`) to keep track of active roles.

use std::collections::HashSet;
use std::process::Command;
use clap::Parser;
use std::fs;
use std::io::{self, Write, BufReader, BufRead};
use shlex; // For safely quoting arguments for shell commands

/// Base URL from which role files (tool lists) are fetched.
const BASE_RAW_URL: &str = "https://raw.githubusercontent.com/jakubGodula/cyber-toolkit/main/roles/";

/// Defines the command-line arguments accepted by the application.
#[derive(Parser, Debug)]
#[clap(author, version, about = "Manages roles and associated tools for Athena OS.", long_about = None)]
struct Cli {
    /// Flag to indicate removal of roles and their unique tools.
    /// If present, the listed `role_files` will be removed.
    #[clap(short, long)]
    remove: bool,

    /// Names of the role files to process (e.g., blue-teamer.txt).
    /// These files are expected to be located in the repository defined by `BASE_RAW_URL`.
    /// - If `--remove` is used, these are the roles to remove from the configuration and system.
    /// - Otherwise (default), these roles are added to the configuration, and their tools are installed/synced.
    #[clap(required = true, num_args = 1..)]
    role_files: Vec<String>,
}

/// Reads the list of currently configured role file names from `~/.roles/roles.cnf`.
/// 
/// Returns a `Vec<String>` of role names. If the config file doesn't exist, an empty vector is returned.
/// Errors during file reading are propagated.
fn read_roles_from_config_file() -> Result<Vec<String>, io::Error> {
    // Construct path to ~/.roles/roles.cnf
    let config_file_path = dirs::home_dir()
        .map(|home_dir| home_dir.join(".roles").join("roles.cnf"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found."))?;

    if !config_file_path.exists() {
        return Ok(Vec::new()); // No config file means no roles configured yet
    }

    let file = fs::File::open(config_file_path)?;
    let reader = BufReader::new(file);
    reader.lines()
        .map(|line| line.map(|s| s.trim().to_string())) // Trim whitespace from each role name
        .filter(|res| match res { // Filter out empty lines
            Ok(s) => !s.is_empty(),
            Err(_) => true, // Keep errors to allow them to be propagated by .collect()
        })
        .collect() // Collect into a Result<Vec<String>, io::Error>
}

/// Writes the given list of role file names to `~/.roles/roles.cnf`, one role per line.
/// 
/// This function overwrites the existing file. It ensures the `~/.roles` directory exists.
/// Errors during directory creation or file writing are propagated.
fn write_roles_to_config_file(roles: &[String]) -> Result<(), io::Error> {
    let config_file_path = dirs::home_dir()
        .map(|home_dir| home_dir.join(".roles").join("roles.cnf"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found."))?;

    // Ensure the .roles directory exists
    if let Some(parent_dir) = config_file_path.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    let mut file = fs::File::create(&config_file_path)?;
    for role_name in roles {
        writeln!(file, "{}", role_name)?;
    }
    println!("Successfully wrote roles to {:?}", config_file_path);
    Ok(())
}

/// Fetches tool lists for the given role file names from the `BASE_RAW_URL`.
/// 
/// For each role file:
/// - Constructs the full URL.
/// - Fetches the content.
/// - Parses each line as a tool name, handling trailing commas and surrounding quotes.
/// - Collects all unique tools from all specified role files.
/// 
/// Returns a `Result` containing a deduplicated `Vec<String>` of tool names, or an error.
async fn fetch_tools_for_role_files(role_files: &[String]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut collected_tools = Vec::new();
    if role_files.is_empty() {
        return Ok(collected_tools); // No roles, no tools
    }

    for role_file_name in role_files {
        let trimmed_role_file_name = role_file_name.trim();
        if trimmed_role_file_name.is_empty() {
            continue; // Skip empty role file names
        }
        let full_tool_list_url = format!("{}{}", BASE_RAW_URL, trimmed_role_file_name);
        println!("Fetching tool list from {}...", full_tool_list_url);

        let response = reqwest::get(&full_tool_list_url).await;
        match response {
            Ok(res) => {
                if !res.status().is_success() {
                    eprintln!("Failed to fetch tool list from {}: HTTP Status {}. Skipping this file.", full_tool_list_url, res.status());
                    continue; // Skip this role file on HTTP error
                }
                let tool_list_text = res.text().await?;
                let tools_from_current_file: Vec<String> = tool_list_text
                    .lines()
                    .map(|line| {
                        // Normalize tool names: trim whitespace, remove trailing commas, and strip surrounding quotes.
                        let s = line.trim();
                        let s_no_comma = s.trim_end_matches(',');
                        let mut final_s = s_no_comma.trim(); // Trim again after comma removal
                        if final_s.len() >= 2 {
                            if (final_s.starts_with('"') && final_s.ends_with('"')) ||
                               (final_s.starts_with('\'') && final_s.ends_with('\'')) {
                                final_s = &final_s[1..final_s.len() - 1]; // Strip quotes
                            }
                        }
                        final_s.to_string()
                    })
                    .filter(|s| !s.is_empty()) // Filter out empty lines after processing
                    .collect();
                
                if tools_from_current_file.is_empty() {
                    println!("No tools found in {}.", full_tool_list_url);
                } else {
                    println!("Found tools in {}: {:?}", full_tool_list_url, tools_from_current_file);
                    collected_tools.extend(tools_from_current_file);
                }
            }
            Err(e) => {
                // Log error and continue with other role files if possible
                eprintln!("Error fetching tool list from {}: {}. Skipping this file.", full_tool_list_url, e);
                continue;
            }
        }
    }

    // Deduplicate the final list of tools
    if !collected_tools.is_empty() {
        collected_tools.sort_unstable();
        collected_tools.dedup();
    }
    Ok(collected_tools)
}

/// Executes a pacman command (`-Syu` or `-Runs`) for the given list of tools.
/// 
/// Uses `pkexec` to run `sudo pacman`.
/// Tools are quoted using `shlex` for safe shell execution.
/// Pacman flags `--confirm --overwrite` are used as per user specification.
/// 
/// # Arguments
/// * `operation_flag`: Either "Syu" (for install/update) or "Rcns" (for remove - mapped to -Runs for pacman).
/// * `tools`: A slice of tool names to process.
/// 
/// Returns `Ok(())` on success, or an error if the pacman command fails.
async fn run_pacman_command(operation_flag: &str, tools: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if tools.is_empty() {
        println!("No tools specified for pacman {} operation.", operation_flag);
        return Ok(());
    }

    // Map the logical operation_flag to the actual pacman argument string
    let pacman_op_arg = match operation_flag {
        "Syu" => "-Syu",
        "Rcns" => "-Runs", // Maps to -Runs for pacman as per user's previous edit
        _ => return Err(Box::from(format!("Unsupported pacman operation: {}", operation_flag))),
    };

    let mut quoted_tools: Vec<String> = Vec::with_capacity(tools.len());
    for tool in tools {
        match shlex::try_quote(tool) {
            Ok(quoted_tool) => quoted_tools.push(quoted_tool.into_owned()),
            Err(e) => {
                eprintln!("Warning: Could not quote tool name '{}' due to error: {}. Skipping this tool.", tool, e);
                // Optionally, we could add it to a list of skipped tools and report at the end.
            }
        }
    }

    if quoted_tools.is_empty() {
        println!("No tools could be safely quoted for pacman {} operation.", operation_flag);
        return Ok(());
    }

    let tools_string = quoted_tools.join(" ");
    // Using --confirm --overwrite as per user's edit
    let command_str = format!("sudo pacman {} --confirm --overwrite {}", pacman_op_arg, tools_string);
    
    println!("Attempting to execute: {}", command_str);
    println!("Note: --confirm flag requires manual 'y/N' input for pacman operations.");

    let status = Command::new("pkexec")
        .arg("sh")
        .arg("-c")
        .arg(&command_str)
        .status()?;

    if status.success() {
        println!("Pacman {} operation completed successfully for tools: {:?}", operation_flag, tools);
    } else {
        eprintln!("Pacman {} operation failed. Exit code: {:?}. Command: {}", operation_flag, status.code(), command_str);
        return Err(Box::from(format!("Pacman {} operation failed for tools: {:?}", operation_flag, tools)));
    }
    Ok(())
}

/// Handles the logic for adding roles and syncing tools.
/// 
/// - Reads existing roles from `~/.roles/roles.cnf`.
/// - Appends new roles provided in `roles_to_add_from_args`.
/// - Deduplicates and writes the updated list back to the config file.
/// - Fetches tools for *all* currently configured roles.
/// - Installs/updates these tools using `pacman -Syu`.
async fn handle_add_command(roles_to_add_from_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_roles = read_roles_from_config_file().unwrap_or_else(|e| {
        eprintln!("Warning: Could not read existing roles config: {}. Starting with an empty list.", e);
        Vec::new()
    });

    // Add new roles from arguments
    for role_arg in roles_to_add_from_args {
        let trimmed_role = role_arg.trim().to_string();
        if !trimmed_role.is_empty() {
            current_roles.push(trimmed_role);
        }
    }
    // Deduplicate and save updated roles list
    current_roles.sort_unstable();
    current_roles.dedup();
    write_roles_to_config_file(&current_roles)?;

    println!("\nFetching all tools for currently configured roles to ensure system is up to date...");
    let all_tools_for_configured_roles = fetch_tools_for_role_files(&current_roles).await?;
    
    if !all_tools_for_configured_roles.is_empty() {
        println!("\nTotal unique tools to install/update from all configured roles: {:?}", all_tools_for_configured_roles);
        run_pacman_command("Syu", &all_tools_for_configured_roles).await?;
    } else {
        println!("No tools to install/update based on the current configuration.");
    }
    Ok(())
}

/// Handles the logic for removing roles and their unique tools.
/// 
/// - Reads existing roles from `~/.roles/roles.cnf`.
/// - Identifies roles to keep and roles to remove based on `roles_to_remove_from_args`.
/// - Fetches tools for kept roles and for removed roles separately.
/// - Determines tools unique to the removed roles (tools not present in any kept role).
/// - Uninstalls these unique tools using `pacman -Runs`.
/// - Writes the updated list of (kept) roles back to the config file.
async fn handle_remove_command(roles_to_remove_from_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let configured_roles_before_removal = read_roles_from_config_file().unwrap_or_else(|e| {
        eprintln!("Warning: Could not read existing roles config: {}. Assuming no roles were configured.", e);
        Vec::new()
    });

    if configured_roles_before_removal.is_empty() {
        println!("No roles currently configured. Nothing to remove.");
        return Ok(());
    }

    // Determine which roles to keep and which are actually being removed
    let roles_to_remove_set: HashSet<_> = roles_to_remove_from_args.iter().map(|s| s.trim().to_string()).collect();
    let roles_to_keep: Vec<String> = configured_roles_before_removal
        .iter()
        .filter(|r| !roles_to_remove_set.contains(*r))
        .cloned()
        .collect();
    
    let roles_actually_removed: Vec<String> = configured_roles_before_removal
        .iter()
        .filter(|r| roles_to_remove_set.contains(*r))
        .cloned()
        .collect();

    if roles_actually_removed.is_empty() {
        println!("None of the specified roles to remove were found in the current configuration.");
        write_roles_to_config_file(&roles_to_keep)?; // Still write, to ensure config is clean
        return Ok(());
    }

    println!("Roles to keep: {:?}", roles_to_keep);
    println!("Roles being removed: {:?}", roles_actually_removed);

    // Fetch tools for kept roles and removed roles
    let tools_for_kept_roles = fetch_tools_for_role_files(&roles_to_keep).await?;
    let tools_for_removed_roles = fetch_tools_for_role_files(&roles_actually_removed).await?;

    // Identify tools unique to the removed roles
    let tools_for_kept_roles_set: HashSet<_> = tools_for_kept_roles.into_iter().collect();
    let mut tools_to_uninstall = Vec::new();

    for tool in tools_for_removed_roles {
        if !tools_for_kept_roles_set.contains(&tool) {
            tools_to_uninstall.push(tool);
        }
    }

    // Uninstall unique tools
    if !tools_to_uninstall.is_empty() {
        println!("\nTools to uninstall (unique to removed roles): {:?}", tools_to_uninstall);
        run_pacman_command("Rcns", &tools_to_uninstall).await?;
    } else {
        println!("No tools to uninstall. Either removed roles had no unique tools or no tools at all.");
    }

    // Update the configuration file with the kept roles
    write_roles_to_config_file(&roles_to_keep)?;
    println!("Configuration updated. Roles {:?} removed.", roles_actually_removed);
    Ok(())
}

/// Main entry point of the application.
/// 
/// Parses command-line arguments and dispatches to either `handle_add_command`
/// or `handle_remove_command` based on the presence of the `--remove` flag.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse(); // Parse command-line arguments

    // Dispatch based on the --remove flag
    if cli.remove {
        println!("Executing REMOVE command for roles: {:?}", cli.role_files);
        handle_remove_command(&cli.role_files).await?;
    } else {
        println!("Executing ADD/SYNC command for roles: {:?}", cli.role_files);
        handle_add_command(&cli.role_files).await?;
    }

    println!("\n--- Operation finished ---");
    Ok(())
}
