use std::collections::HashSet;
use std::process::Command;
use clap::Parser;
use std::fs;
use std::io::{self, Write, BufReader, BufRead};
use shlex;

const BASE_RAW_URL: &str = "https://raw.githubusercontent.com/jakubGodula/cyber-toolkit/main/roles/";

#[derive(Parser, Debug)]
#[clap(author, version, about = "Manages roles and associated tools for Athena OS.", long_about = None)]
struct Cli {
    /// Flag to indicate removal of roles and their unique tools.
    #[clap(short, long)]
    remove: bool,

    /// Names of the role files to process (e.g., blue-teamer.txt).
    /// If --remove is used, these are the roles to remove.
    /// Otherwise, these are the roles to add/ensure are present.
    #[clap(required = true, num_args = 1..)]
    role_files: Vec<String>,
}

fn read_roles_from_config_file() -> Result<Vec<String>, io::Error> {
    let config_file_path = dirs::home_dir()
        .map(|home_dir| home_dir.join(".roles").join("roles.cnf"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found."))?;

    if !config_file_path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(config_file_path)?;
    let reader = BufReader::new(file);
    reader.lines()
        .map(|line| line.map(|s| s.trim().to_string()))
        .filter(|res| match res {
            Ok(s) => !s.is_empty(),
            Err(_) => true, 
        })
        .collect()
}

fn write_roles_to_config_file(roles: &[String]) -> Result<(), io::Error> {
    let config_file_path = dirs::home_dir()
        .map(|home_dir| home_dir.join(".roles").join("roles.cnf"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found."))?;

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

async fn fetch_tools_for_role_files(role_files: &[String]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut collected_tools = Vec::new();
    if role_files.is_empty() {
        return Ok(collected_tools);
    }

    for role_file_name in role_files {
        let full_tool_list_url = format!("{}{}", BASE_RAW_URL, role_file_name.trim());
        println!("Fetching tool list from {}...", full_tool_list_url);

        let response = reqwest::get(&full_tool_list_url).await;
        match response {
            Ok(res) => {
                if !res.status().is_success() {
                    eprintln!("Failed to fetch tool list from {}: HTTP Status {}. Skipping this file.", full_tool_list_url, res.status());
                    continue;
                }
                let tool_list_text = res.text().await?;
                let tools_from_current_file: Vec<String> = tool_list_text
                    .lines()
                    .map(|line| {
                        let s = line.trim();
                        let s_no_comma = s.trim_end_matches(',');
                        let mut final_s = s_no_comma.trim();
                        if final_s.len() >= 2 {
                            if (final_s.starts_with('"') && final_s.ends_with('"')) ||
                               (final_s.starts_with('\'') && final_s.ends_with('\'')) {
                                final_s = &final_s[1..final_s.len() - 1];
                            }
                        }
                        final_s.to_string()
                    })
                    .filter(|s| !s.is_empty())
                    .collect();
                
                if tools_from_current_file.is_empty() {
                    println!("No tools found in {}.", full_tool_list_url);
                } else {
                    println!("Found tools in {}: {:?}", full_tool_list_url, tools_from_current_file);
                    collected_tools.extend(tools_from_current_file);
                }
            }
            Err(e) => {
                eprintln!("Error fetching tool list from {}: {}. Skipping this file.", full_tool_list_url, e);
                continue;
            }
        }
    }

    if !collected_tools.is_empty() {
        collected_tools.sort_unstable();
        collected_tools.dedup();
    }
    Ok(collected_tools)
}

async fn run_pacman_command(operation_flag: &str, tools: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if tools.is_empty() {
        println!("No tools specified for pacman {} operation.", operation_flag);
        return Ok(());
    }

    let quoted_tools: Vec<String> = tools.iter().map(|tool| shlex::quote(tool).into_owned()).collect();
    let tools_string = quoted_tools.join(" ");
    let pacman_op_arg = match operation_flag {
        "Syu" => "-Syu",
        "Rcns" => "-Runs",
        _ => return Err(Box::from(format!("Unsupported pacman operation: {}", operation_flag)))
    };

    let command_str = format!("sudo pacman {} --confirm --overwrite {}", pacman_op_arg, tools_string);
    println!("Attempting to execute: {}", command_str);

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

async fn handle_add_command(roles_to_add_from_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_roles = read_roles_from_config_file().unwrap_or_else(|e| {
        eprintln!("Warning: Could not read existing roles config: {}. Starting with an empty list.", e);
        Vec::new()
    });

    for role_arg in roles_to_add_from_args {
        let trimmed_role = role_arg.trim().to_string();
        if !trimmed_role.is_empty() {
            current_roles.push(trimmed_role);
        }
    }
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

async fn handle_remove_command(roles_to_remove_from_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let configured_roles_before_removal = read_roles_from_config_file().unwrap_or_else(|e| {
        eprintln!("Warning: Could not read existing roles config: {}. Assuming no roles were configured.", e);
        Vec::new()
    });

    if configured_roles_before_removal.is_empty() {
        println!("No roles currently configured. Nothing to remove.");
        return Ok(());
    }

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
        write_roles_to_config_file(&roles_to_keep)?;
        return Ok(());
    }

    println!("Roles to keep: {:?}", roles_to_keep);
    println!("Roles being removed: {:?}", roles_actually_removed);

    let tools_for_kept_roles = fetch_tools_for_role_files(&roles_to_keep).await?;
    let tools_for_removed_roles = fetch_tools_for_role_files(&roles_actually_removed).await?;

    let tools_for_kept_roles_set: HashSet<_> = tools_for_kept_roles.into_iter().collect();
    let mut tools_to_uninstall = Vec::new();

    for tool in tools_for_removed_roles {
        if !tools_for_kept_roles_set.contains(&tool) {
            tools_to_uninstall.push(tool);
        }
    }

    if !tools_to_uninstall.is_empty() {
        println!("\nTools to uninstall (unique to removed roles): {:?}", tools_to_uninstall);
        run_pacman_command("Rcns", &tools_to_uninstall).await?;
    } else {
        println!("No tools to uninstall. Either removed roles had no unique tools or no tools at all.");
    }

    write_roles_to_config_file(&roles_to_keep)?;
    println!("Configuration updated. Roles {:?} removed.", roles_actually_removed);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

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
