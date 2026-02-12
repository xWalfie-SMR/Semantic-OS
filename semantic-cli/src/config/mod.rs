// config/mod.rs
// Handles loading, building, and saving the user's SemanticOS configuration.
// Config lives at ~/.config/semantic/config.toml

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// -- config structs (mirrors config.toml layout) --

/// Top-level config. Serializes directly to/from config.toml.
#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticConfig {
    pub general: GeneralConfig,
    pub shells: ShellConfig,
    pub commands: HashMap<String, String>,
    pub paths: HashMap<String, String>,
}

/// User preferences for command and folder styles.
#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub command_style: String,
    pub folder_style: String,
}

/// Shell-related settings: which shell, which are enabled, what to do on new installs.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShellConfig {
    pub default: String,
    pub enabled: Vec<String>,
    pub on_new_shell: String,
}

impl SemanticConfig {
    /// Build a config from the TUI installer selections.
    /// Picks the right command/path mappings based on the chosen styles.
    pub fn from_selections(
        shell: &str,
        command_style: &str,
        folder_style: &str,
        on_new_shell: &str,
    ) -> Self {
        // pick command mappings based on style
        let commands = match command_style {
            "natural" => natural_commands(),
            "verbose" => verbose_commands(),
            _ => traditional_commands(),
        };

        // pick path mappings based on style
        let paths = match folder_style {
            "natural" => natural_paths(),
            "verbose" => verbose_paths(),
            _ => traditional_paths(),
        };

        SemanticConfig {
            general: GeneralConfig {
                command_style: command_style.to_string(),
                folder_style: folder_style.to_string(),
            },
            shells: ShellConfig {
                default: shell.to_string(),
                enabled: vec![shell.to_string()],
                on_new_shell: on_new_shell.to_string(),
            },
            commands,
            paths,
        }
    }

    /// Load config from ~/.config/semantic/config.toml.
    /// Returns an error if the file doesn't exist or can't be parsed.
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::config_path();
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("{}: {e}", config_path.display()))?;
        let config: SemanticConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Write the config to ~/.config/semantic/config.toml.
    /// Creates the directory if it doesn't exist.
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_dir = config_dir();
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.toml");
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;

        Ok(())
    }

    /// Returns the full path to config.toml (for display purposes).
    pub fn config_path() -> PathBuf {
        config_dir().join("config.toml")
    }
}

/// Resolves ~/.config/semantic/ using the dirs crate.
fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("semantic")
}

// -- command mappings --
// Each style returns a map of semantic_command -> real_command.
// These match the templates in templates/*.toml.

fn natural_commands() -> HashMap<String, String> {
    HashMap::from([
        ("goto".into(), "cd".into()),
        ("back".into(), "cd ..".into()),
        ("list".into(), "ls -la".into()),
        ("delete".into(), "rm -rf".into()),
        ("copy".into(), "cp -r".into()),
        ("move".into(), "mv".into()),
        ("install".into(), "sudo pacman -S".into()),
        ("remove".into(), "sudo pacman -R".into()),
        ("update".into(), "sudo pacman -Syu".into()),
    ])
}

fn verbose_commands() -> HashMap<String, String> {
    HashMap::from([
        ("go-to".into(), "cd".into()),
        ("go-back".into(), "cd ..".into()),
        ("list-files".into(), "ls -la".into()),
        ("delete-file".into(), "rm -rf".into()),
        ("copy-file".into(), "cp -r".into()),
        ("move-file".into(), "mv".into()),
        ("install-package".into(), "sudo pacman -S".into()),
        ("remove-package".into(), "sudo pacman -R".into()),
        ("update-system".into(), "sudo pacman -Syu".into()),
    ])
}

fn traditional_commands() -> HashMap<String, String> {
    // identity mappings — real commands map to themselves
    HashMap::from([
        ("cd".into(), "cd".into()),
        ("ls".into(), "ls".into()),
        ("rm".into(), "rm".into()),
        ("cp".into(), "cp".into()),
        ("mv".into(), "mv".into()),
        ("pacman".into(), "pacman".into()),
    ])
}

// -- path mappings --
// Each style returns a map of virtual_path -> real_path.
// Used by the FUSE layer to remap directory names.

fn natural_paths() -> HashMap<String, String> {
    HashMap::from([
        ("/apps".into(), "/usr/bin".into()),
        ("/settings".into(), "/etc".into()),
        ("/logs".into(), "/var/log".into()),
    ])
}

fn verbose_paths() -> HashMap<String, String> {
    HashMap::from([
        ("/user/applications".into(), "/usr/bin".into()),
        ("/configuration".into(), "/etc".into()),
        ("/system-logs".into(), "/var/log".into()),
    ])
}

fn traditional_paths() -> HashMap<String, String> {
    // no remapping — use real paths as-is
    HashMap::new()
}
