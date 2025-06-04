use std::collections::HashSet;
use std::process::Command;
// clap::Parser is for the binary, not usually the lib, unless lib exposes CLI building blocks.
// For now, keep clap in main.rs.
use std::fs;
use std::io::{self, Write, BufReader, BufRead};
use shlex; // For safely quoting arguments for shell commands
use serde::Deserialize; // Added for GitHub API response parsing
use std::env; // For accessing current executable path and arguments
use std::path::Path;

// Consider if these constants are truly lib-level or should be passed from main.
// REPO_URL seems like a good candidate for the library.
pub const REPO_URL: &str = "https://raw.githubusercontent.com/jakubGodula/cyber-toolkit/main/roles/";

#[derive(Deserialize, Debug)] // Made public if it needs to be part of public API, otherwise keep private
pub struct GitHubContentItem {
    name: String,
    #[serde(rename = "type")]
    item_type: String,
}

pub async fn display_available_roles() -> Result<(), Box<dyn std::error::Error>> {
    let role_names_url = format!("{}role_names", REPO_URL);
    let response = reqwest::get(&role_names_url).await?;
    if !response.status().is_success() {
        eprintln!("Error: Could not fetch the list of defined roles from {}. HTTP Status: {}", role_names_url, response.status());
        return Ok(());
    }
    let role_names_list_content = response.text().await?;
    print!("{}", role_names_list_content);
    Ok(())
}

pub async fn display_available_roles_and_tools() -> Result<(), Box<dyn std::error::Error>> {
    let role_names_url = format!("{}role_names", REPO_URL);
    let response = reqwest::get(&role_names_url).await?;
    if !response.status().is_success() {
        eprintln!("Error: Could not fetch the list of defined roles from {}. HTTP Status: {}", role_names_url, response.status());
        return Ok(());
    }
    let role_names_list_content = response.text().await?;
    let role_files_to_check: Vec<String> = role_names_list_content   
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if role_files_to_check.is_empty() {
        println!("No roles are defined in the central 'role_names' file.");
        return Ok(());
    }

    for role_file_name in role_files_to_check {
        print!("{}:", role_file_name);
        match fetch_tools_for_role_files(&[role_file_name.clone()]).await {
            Ok(tools) => {
                if tools.is_empty() {
                    print!("No tools listed for this role");
                } else {
                    for tool_name in tools {
                        print!("\n  {}", tool_name);
                        // Pacman -Ss logic was commented out, keeping it that way
                    }
                }
            }
            Err(e) => {
                eprintln!("  Error fetching tool list for role '{}': {}", role_file_name, e);
            }
        }
        println!();
    }
    Ok(())
}

pub fn read_roles_from_config_file() -> Result<Vec<String>, io::Error> {
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

pub fn write_roles_to_config_file(roles: &[String]) -> Result<(), io::Error> {
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
    println!("Successfully wrote roles:\n{:?} to {:?}", roles, config_file_path);
    Ok(())
}

pub async fn fetch_tools_for_role_files(role_files: &[String]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut collected_tools = Vec::new();
    if role_files.is_empty() {
        return Ok(collected_tools);
    }

    for role_file_name in role_files {
        let trimmed_role_file_name = role_file_name.trim();
        if trimmed_role_file_name.is_empty() {
            eprintln!("No valid role name was provided! Please refer to the roles available at the following url: \n{}",REPO_URL);
            continue;
        }
        let full_tool_list_url = format!("{}{}", REPO_URL, trimmed_role_file_name);
        
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
                    collected_tools.extend(tools_from_current_file);
                }
            }
            Err(e) => {
                eprintln!("Error fetching tool list from {}: {}. Halting further fetching for this operation.", full_tool_list_url, e);
                break; 
            }
        }
    }
    if !collected_tools.is_empty() {
        collected_tools.sort_unstable();
        collected_tools.dedup();
    }
    Ok(collected_tools)
}

pub async fn run_pacman_command(operation_flag: &str, tools: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if tools.is_empty() {
        println!("No tools specified for pacman {} operation.", operation_flag);
        return Ok(());
    }
    let pacman_op_arg = match operation_flag {
        "Syu" => "-Syu --noconfirm --needed",
        "Rcns" => "-Runs --noconfirm", 
        _ => return Err(Box::from(format!("Unsupported pacman operation: {}", operation_flag))),
    };

    let mut quoted_tools_bulk: Vec<String> = Vec::with_capacity(tools.len());
    for tool in tools {
        match shlex::try_quote(tool) {
            Ok(quoted_tool) => quoted_tools_bulk.push(quoted_tool.into_owned()),
            Err(e) => {
                eprintln!("Warning: Could not quote tool name '{}' for bulk operation due to error: {}. It might be skipped or fail if processed individually.", tool, e);
            }
        }
    }

    if quoted_tools_bulk.is_empty() && !tools.is_empty() {
        eprintln!("No tools could be safely quoted for pacman {} bulk operation. Attempting individual operations.", operation_flag);
    } else if !quoted_tools_bulk.is_empty() {
        let tools_string_bulk = quoted_tools_bulk.join(" ");
        let command_str_bulk = format!("pacman {} {}", pacman_op_arg, tools_string_bulk); // Assuming root from main
        
        println!("Attempting bulk pacman {} operation for: {:?}", operation_flag, tools);
        let status_bulk = Command::new("sh")
            .arg("-c")
            .arg(&command_str_bulk)
            .status()?;

        if status_bulk.success() {
            println!("Bulk pacman {} operation completed successfully for all tools.", operation_flag);
            return Ok(());
        } else {
            eprintln!("Bulk pacman {} operation failed (Exit code: {:?}). Command: {}. Attempting individual operations for each tool.", operation_flag, status_bulk.code(), command_str_bulk);
        }
    }
    
    println!("Processing tools individually...");
    let mut all_individual_successful = true;
    let mut successful_individual_ops = 0;
    let mut failed_individual_ops = Vec::new();

    for tool_name in tools {
        match shlex::try_quote(tool_name) {
            Ok(quoted_tool_single) => {
                let command_str_single = format!("pacman {} {}", pacman_op_arg, quoted_tool_single); // Assuming root
                let status_single = Command::new("sh")
                    .arg("-c")
                    .arg(&command_str_single)
                    .status()?;

                if status_single.success() {
                    println!("Pacman {} operation successful for tool: {}", operation_flag, tool_name);
                    successful_individual_ops += 1;
                } else {
                    failed_individual_ops.push(tool_name.clone());
                    all_individual_successful = false;
                }
            },
            Err(e) => {
                eprintln!("Error: Could not quote tool name '{}' for individual pacman operation: {}. Skipping this tool.", tool_name, e);
                all_individual_successful = false;
            }
        }
    }

    if all_individual_successful {
        println!("All individual pacman {} operations completed successfully.", operation_flag);
        Ok(())
    } else {
        if successful_individual_ops > 0 {
             eprintln!("Some individual pacman {} operations failed: \n{:?}, but other {} succeeded.", operation_flag, failed_individual_ops, successful_individual_ops);
        } else {
             eprintln!("All individual pacman {} operations failed.", operation_flag);
        }
        Err(Box::from(format!("One or more pacman {} operations failed during individual processing after bulk attempt.", operation_flag)))
    }
}

pub async fn handle_add_command(roles_to_add_from_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
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
    

    println!("\nFetching all tools for currently configured roles to ensure system is up to date...");
    let all_tools_for_configured_roles = fetch_tools_for_role_files(&current_roles).await?;
    
    if !all_tools_for_configured_roles.is_empty() {
        println!("\nTotal unique tools to install/update from all configured roles: {:?}", all_tools_for_configured_roles);
        run_pacman_command("Syu", &all_tools_for_configured_roles).await?;
    } else {
        println!("No tools to install/update based on the current configuration.");
    }
    write_roles_to_config_file(&current_roles)?;
    Ok(())
}

pub async fn handle_update_command(roles_to_set_from_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    println!("Executing UPDATE command. Target roles to set: {:?}", roles_to_set_from_args);
    // Check if any of the roles_to_set_from_args are not in config
    let current_roles = read_roles_from_config_file().unwrap_or_else(|e| {
        eprintln!("Warning: Could not read existing roles config: {}. Starting with an empty list.", e);
        Vec::new()
    });

    let current_roles_set: HashSet<_> = current_roles.iter().cloned().collect();
    let roles_to_set_set: HashSet<_> = roles_to_set_from_args.iter().map(|s| s.trim().to_string()).collect();

    if !roles_to_set_set.is_subset(&current_roles_set) {
        // Some roles to set are not in config, need to add them first
        println!("Some target roles are not currently configured. Adding them first...");
        handle_add_command(roles_to_set_from_args).await?;
        handle_remove_command(&current_roles).await?;
    }

    let current_roles_from_config = read_roles_from_config_file().unwrap_or_else(|e| {
        eprintln!("Warning: Could not read existing roles config for update: {}. Assuming empty.", e);
        Vec::new()
    });
    println!("Roles currently in config: {:?}", current_roles_from_config);
    /* */
    let mut target_roles: Vec<String> = roles_to_set_from_args
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    target_roles.sort_unstable();
    target_roles.dedup();
    println!("Target roles for system state: {:?}", target_roles);

    let target_roles_set: HashSet<_> = target_roles.iter().cloned().collect();
    let roles_to_remove_calculated: Vec<String> = current_roles_from_config
        .iter()
        .filter(|role| !target_roles_set.contains(*role))
        .cloned()
        .collect();
    println!("Roles to be explicitly removed (if they exist and are not part of target): {:?}", roles_to_remove_calculated);

    println!("\nStep 1: Ensuring all target roles and their tools are present...");
    handle_add_command(&target_roles).await?;

    if !roles_to_remove_calculated.is_empty() {
        println!("\nStep 2: Removing roles (and their unique tools) that are no longer in the target set...");
        handle_remove_command(&roles_to_remove_calculated).await?;
    } else {
        println!("\nStep 2: No roles to remove from the previous configuration that are not in the target set.");
    }

    println!("\nUpdate command finished. Roles configuration should now reflect target roles: {:?}", target_roles);
    Ok(())
}

pub async fn handle_remove_command(roles_to_remove_from_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
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
    println!("Roles being processed for removal: {:?}", roles_actually_removed);

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
        run_pacman_command("Rcns", &tools_to_uninstall).await?;
    } else {
        println!("No tools to uninstall. Either removed roles had no unique tools or no tools at all.");
    }

    write_roles_to_config_file(&roles_to_keep)?;
    println!("Configuration updated. Roles {:?} removed.", roles_actually_removed);
    Ok(())
}

pub async fn handle_current_command() -> Result<(), Box<dyn std::error::Error>> {
    let current_roles = read_roles_from_config_file().unwrap_or_else(|e| {
        eprintln!("Warning: Could not read existing roles config: {}. Assuming no roles were configured.", e);
        Vec::new()
    });
    println!("Current roles: {:?}", current_roles);
    Ok(())
}
pub fn check_if_user_is_root() -> bool {
    match Command::new("id").arg("-u").output() {
        Ok(output) => match String::from_utf8(output.stdout) {
            Ok(stdout_str) => stdout_str.trim() == "0",
            Err(_) => false, 
        },
        Err(_) => false, 
    }
}

pub fn elevate_to_root() -> Result<(), Box<dyn std::error::Error>> {
    println!("Root privileges are required. Attempting to re-run with sudo...");
    let current_exe = env::current_exe()?;
    let args: Vec<String> = env::args().skip(1).collect();
    
    let mut sudo_cmd = Command::new("sudo");
    sudo_cmd.arg(current_exe);
    sudo_cmd.args(args);

    let status = sudo_cmd.status()?;

    std::process::exit(status.code().unwrap_or(if status.success() { 0 } else { 1 }));
}
