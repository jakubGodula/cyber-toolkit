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
    target/debug/cyber-toolkit blue-teamer.txt web-tools.txt
    ```

2.  **Remove Roles:**
    This command removes `blue-teamer.txt` from `~/.roles/roles.cnf`. It then identifies tools that were unique to `blue-teamer.txt` (and not part of any other roles remaining in `roles.cnf`) and uninstalls them using `sudo pacman -Runs --confirm --overwrite`.

    ```bash
    target/debug/cyber-toolkit -r blue-teamer.txt
    ```

    To remove multiple roles:
    ```bash
    target/debug/cyber-toolkit --remove blue-teamer.txt old-role.txt
    ```

## Tool File Format

Role files (e.g., `blue-teamer.txt`) hosted in the GitHub repository should list one package name per line. The parser handles:
- Leading/trailing whitespace.
- Trailing commas.
- Tool names enclosed in matching single (`'`) or double (`"`) quotes.

**Example `my-role.txt`:**

```
package1
package2,
'package3 with spaces'
  "package4",  
```

This would be parsed as `package1`, `package2`, `package3 with spaces`, and `package4`. 