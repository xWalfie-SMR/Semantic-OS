# SemanticOS

An Arch-based Linux distro with a semantic layer on top.
Instead of memorizing cryptic commands, you choose how you talk to your system.
The real Linux filesystem stays untouched underneath.

## How it works

The semantic layer sits between you and the OS:

- You type human-readable commands
- The translation layer converts them to real Linux commands
- The real filesystem stays untouched

## Features

- **FUSE virtual filesystem** - see /apps instead of /usr/bin, without touching the real filesystem
- **Shell-agnostic command translation** - works in any shell
- **Configurable command styles** - natural, traditional, verbose, or fully custom
- **Per-user configs** - each user picks their own style
- **Rust-based TUI installer** - walks you through shell, style, and preference setup
- **Pacman hooks** - detects when a new shell is installed
- **Beginner friendly** - for users who are starting out with linux, but doesn't get in your way if you know what you're doing

## Command Styles

### Natural

    goto /apps
    list files
    install firefox
    delete myfile

### Traditional

    cd /usr/bin
    ls
    sudo pacman -S firefox
    rm myfile

### Verbose

    go-to /user/applications
    list-files
    install-package firefox
    delete-file myfile

### Custom

Define your own. Your system, your language.

## Folder Styles

| Natural | Traditional | Verbose |
|---------|-------------|---------|
| /apps | /usr/bin | /user/applications |
| /settings | /etc | /configuration |
| /logs | /var/log | /system-logs |

Or make your own.

## Installation

The TUI installer asks you:

- Default shell
- Command style
- Folder style
- New shell behavior (auto-setup/notify/ignore)

All choices are per-user and changeable after install.

## Config

Located at ~/.config/semantic/config.toml

    [general]
    command_style = "natural"
    folder_style = "natural"

    [shells]
    default = "fish"
    enabled = ["fish", "bash"]
    on_new_shell = "notify"

    [commands]
    goto = "cd"
    back = "cd .."
    list = "ls -la"
    install = "sudo pacman -S"

    [paths]
    "/apps" = "/usr/bin"
    "/settings" = "/etc"
    "/logs" = "/var/log"

## Tech Stack

| Component | Tool |
|-----------|------|
| Base | Arch Linux |
| Installer | Rust (ratatui) |
| FUSE layer | Rust (fuser) |
| Shell integration | Rust binary + shell hooks |
| Config format | TOML |
| ISO building | archiso |

## Project Structure

    Semantic-OS/
    ├── README.md
    ├── LICENSE
    ├── semantic-cli/        # core CLI tool (Rust)
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── main.rs
    │   │   ├── tui/
    │   │   ├── config/
    │   │   ├── shell/
    │   │   └── fuse/
    │   └── templates/
    │       ├── natural.toml
    │       ├── minimal.toml
    │       └── verbose.toml
    ├── iso/                 # ISO build configs (archiso)
    ├── packages/            # packaging and distribution
    └── docs/                # documentation

## Status

Early development. Building the core semantic binary.

## License

[GNU General Public License V2.0](LICENSE)
