use std::process::Command;
use clap::Parser;
use std::fs;
use std::io::Write;

const BASE_RAW_URL: &str = "https://raw.githubusercontent.com/jakubGodula/cyber-toolkit/main/roles/";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Names of the tool list files (e.g., blue-teamer.txt, red-teamer.txt)
    #[clap(required = true, num_args = 1..)]
    tool_files: Vec<String>,
}

fn write_roles_to_config(tool_files: &[String]) -> Result<(), std::io::Error> {
    if let Some(home_dir) = dirs::home_dir() {
        let config_dir = home_dir.join(".roles");
        fs::create_dir_all(&config_dir)?;
        let config_file_path = config_dir.join("roles.cnf");

        let mut file = fs::File::create(config_file_path)?;
        for tool_file_name in tool_files {
            writeln!(file, "{}", tool_file_name)?;
        }
        println!("Successfully wrote roles to ~/.roles/roles.cnf");
    } else {
        eprintln!("Error: Could not determine home directory. Roles not saved to config.");
        // Optionally, return an error here if this is critical
        // return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found"));
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Write the provided tool file names to the config file
    if let Err(e) = write_roles_to_config(&args.tool_files) {
        eprintln!("Warning: Could not write roles to config file: {}", e);
        // Continue execution even if config writing fails
    }

    let mut all_tools: Vec<String> = Vec::new();

    for tool_file_name in args.tool_files {
        let full_tool_list_url = format!("{}{}", BASE_RAW_URL, tool_file_name);
        println!("Fetching tool list from {}...", full_tool_list_url);

        let response = reqwest::get(&full_tool_list_url).await;
        
        match response {
            Ok(res) => {
                if !res.status().is_success() {
                    eprintln!("Failed to fetch tool list from {}: HTTP Status {}", full_tool_list_url, res.status());
                    continue; // Skip to the next file
                }

                let tool_list_text = res.text().await?;
                let tools_from_current_file: Vec<String> = tool_list_text
                    .lines()
                    .map(|line| line.trim().trim_end_matches(',').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                if tools_from_current_file.is_empty() {
                    println!("No tools found in {}.", full_tool_list_url);
                } else {
                    println!("Found tools in {}: {:?}", full_tool_list_url, tools_from_current_file);
                    all_tools.extend(tools_from_current_file);
                }
            },
            Err(e) => {
                eprintln!("Error fetching tool list from {}: {}", full_tool_list_url, e);
                continue; // Skip to the next file
            }
        }
    }

    if all_tools.is_empty() {
        println!("No tools found in any of the provided files.");
        return Ok(());
    }

    all_tools.sort_unstable();
    all_tools.dedup();

    println!("\nTotal unique tools to install: {:?}", all_tools);
    println!("Attempting to install tools using pacman...");

    let tools_string = all_tools.join(" ");
    let command_str = format!("sudo pacman -S --noconfirm {}", tools_string);
    println!("Executing: {}", command_str);

    let status = Command::new("pkexec")
        .arg("sh")
        .arg("-c")
        .arg(&command_str)
        .status();

    match status {
        Ok(exit_status) => {
            if exit_status.success() {
                println!("Successfully installed/updated all tools.");
            } else {
                eprintln!("Failed to install/update tools. Exit code: {:?}", exit_status.code());
                eprintln!("Command was: {}", command_str);
                eprintln!("Please check for errors above and try installing manually if needed.");
            }
        }
        Err(e) => {
            eprintln!("Failed to execute command: {}", e);
            eprintln!("Command was: pkexec sh -c \"{}\"", command_str);
            if e.kind() == std::io::ErrorKind::NotFound {
                eprintln!("pkexec command not found. Please ensure Polkit is installed and pkexec is in your PATH.");
            }
            eprintln!("Please try installing manually if needed.");
        }
    }

    println!("\n--- Tool installation process finished ---");
    Ok(())
}
