# Cyber Toolkit

## Function Definitions

### Tool List Providers

#### `ToolListProvider` Trait
```rust
pub trait ToolListProvider {
    async fn fetch_tools(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
}
```

Provider that fetches tools from a Solana program.

### Core Functions

#### `fetch_tools_from_source`
```rust
pub async fn fetch_tools_from_source(source: &str) -> Result<Vec<String>, Box<dyn std::error::Error>>
```
Fetches tools from either GitHub or Solana based on the specified source.

#### `display_available_roles`
```rust
pub async fn display_available_roles() -> Result<(), Box<dyn std::error::Error>>
```
Displays a list of all available roles from the repository.

#### `display_available_roles_and_tools`
```rust
pub async fn display_available_roles_and_tools() -> Result<(), Box<dyn std::error::Error>>
```
Displays all available roles and their associated tools from the repository.

### Configuration Management

#### `read_roles_from_config_file`
```rust
pub fn read_roles_from_config_file() -> Result<Vec<String>, io::Error>
```
Reads the list of currently configured roles from the configuration file.

#### `write_roles_to_config_file`
```rust
pub fn write_roles_to_config_file(roles: &[String]) -> Result<(), io::Error>
```
Writes the list of roles to the configuration file.

### Tool Management

#### `fetch_tools_for_role_files`
```rust
pub async fn fetch_tools_for_role_files(role_files: &[String]) -> Result<Vec<String>, Box<dyn std::error::Error>>
```
Fetches the list of tools for the specified role files.

#### `run_pacman_command`
```rust
pub async fn run_pacman_command(operation_flag: &str, tools: &[String]) -> Result<(), Box<dyn std::error::Error>>
```
Executes a pacman command for the specified tools.

### Command Handlers

#### `handle_add_command`
```rust
pub async fn handle_add_command(roles_to_add_from_args: &[String]) -> Result<(), Box<dyn std::error::Error>>
```
Handles the add command for specified roles.

#### `handle_update_command`
```rust
pub async fn handle_update_command(roles_to_set_from_args: &[String]) -> Result<(), Box<dyn std::error::Error>>
```
Handles the update command to set the system to a specific set of roles.

#### `handle_remove_command`
```rust
pub async fn handle_remove_command(roles_to_remove_from_args: &[String]) -> Result<(), Box<dyn std::error::Error>>
```
Handles the remove command for specified roles.

#### `handle_current_command`
```rust
pub async fn handle_current_command() -> Result<(), Box<dyn std::error::Error>>
```
Handles the current command to display the currently configured roles.

### Privilege Management

#### `check_if_user_is_root`
```rust
pub fn check_if_user_is_root() -> bool
```
Checks if the current user has root privileges.

#### `elevate_to_root`
```rust
pub fn elevate_to_root() -> Result<(), Box<dyn std::error::Error>>
```
Attempts to elevate the program to root privileges using sudo.

### Data Structures

#### `SolanaTool`
```rust
pub struct SolanaTool {
    pub is_initialized: bool,
    pub name: String,
    pub version: String,
    pub role: String,
    pub description: String,
}
```
Represents a tool entry in the Solana program.

#### `GitHubContentItem`
```rust
pub struct GitHubContentItem {
    name: String,
    #[serde(rename = "type")]
    item_type: String,
}
```
Represents a content item from the GitHub API response.

# Cyber Toolkit Manager

A command-line utility to manage collections of cybersecurity tools (roles) on Arch Linux-based systems. 
It fetches tool lists from a predefined GitHub repository, installs them using `pacman`, and manages a local configuration file to keep track of active roles.

## Features

- **Role-based Tool Management**: Organize tools into "roles" defined by text files in a GitHub repository.
- **Add/Sync Roles**: Add new roles to your local configuration. The tool ensures all packages listed in the configured roles are installed or updated.
- **Remove Roles**: Remove roles from your local configuration and uninstall tools that are unique to the removed roles (and not part of any other active role).
- **Configuration File**: Maintains a list of active roles in `~/.roles/roles.cnf`.
- **Pacman Integration**: Uses `pacman` for package installation (`-Syu --confirm --overwrite`) and removal (`-Runs --confirm --overwrite`). Requires `pkexec` for privilege escalation.

## Prerequisites

- Arch Linux or an Arch-based distribution.
- `pacman` package manager.
- `pkexec` (part of Polkit) for running `pacman` with root privileges.
- `git` (for cloning this repository, if applicable).
- `rust` and `cargo` for building the project.

## Setup and Building

1.  **Clone the repository (if you haven't already):**
    ```bash
    # If the cyber-toolkit is part of a larger project, navigate to its directory.
    # Otherwise, clone it:
    # git clone <repository_url>
    # cd cyber-toolkit
    ```

2.  **Build the project:**
    Navigate to the `cyber-toolkit` project directory (e.g., `/home/jakub/athena-welcome-rs/athena-welcome-rs/cyber-toolkit` based on previous context) and run:
    ```bash
    cargo build
    ```
    The executable will be located at `target/debug/cyber-toolkit`.

## Configuration

- **Base URL for Tool Lists**: The program is hardcoded to fetch role files from `https://raw.githubusercontent.com/jakubGodula/cyber-toolkit/main/roles/`. Each role file (e.g., `blue-teamer.txt`) should list one tool per line. Trailing commas and surrounding quotes (single or double) on tool names are automatically handled.
- **Local Role Configuration**: Active roles are stored in `~/.roles/roles.cnf`, one role file name per line.

## Usage

The program is run from the command line. All commands require `pkexec` to interact with `pacman`, which will typically prompt for your password.

**General Syntax:**

```bash
target/debug/cyber-toolkit [OPTIONS] <ROLE_FILE_NAMES...>
```

**Arguments:**

-   `<ROLE_FILE_NAMES...>`: One or more role file names (e.g., `myrole.txt`, `another.txt`). These files should exist in the configured GitHub repository under the `roles/` path.

**Options:**

-   `-r`, `--remove`: If this flag is present, the specified roles will be removed. Otherwise (default behavior), the roles will be added/synced.

### Examples

1.  **Add/Sync Roles:**
    This command adds `blue-teamer.txt` and `web-tools.txt` to `~/.roles/roles.cnf`. It then fetches tool lists for all roles currently in `roles.cnf` and installs/updates them using `sudo pacman -Syu --confirm --overwrite`.

    ```bash
    target/debug/cyber-toolkit blue web
    ```

2.  **Remove Roles:**
    This command removes `blue-teamer.txt` from `~/.roles/roles.cnf`. It then identifies tools that were unique to `blue-teamer.txt` (and not part of any other roles remaining in `roles.cnf`) and uninstalls them using `sudo pacman -Runs --confirm --overwrite`.

    ```bash
    target/debug/cyber-toolkit -r blue
    ```

    To remove multiple roles:
    ```bash
    target/debug/cyber-toolkit --remove blue
    ```


This would be parsed as `package1`, `package2`, `package3 with spaces`, and `package4`. 