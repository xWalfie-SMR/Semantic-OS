// main.rs
// Entry point for the semantic CLI.
//
// Subcommands:
//   (no args)           — launch the TUI installer
//   init                — print shell aliases to stdout (user evals this)
//   translate <cmd> ... — look up a semantic command and run the real one

mod config;
mod shell;
mod tui;

use std::env;
use std::process::{Command, exit};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.first().map(|s| s.as_str()) {
        // no args — run the TUI installer
        None => tui::run(),

        // print shell init code to stdout
        Some("init") => cmd_init(),

        // translate and execute a semantic command
        Some("translate") => cmd_translate(&args[1..]),

        // unknown subcommand
        Some(other) => {
            eprintln!("Unknown command: {other}");
            eprintln!("Usage: semantic [init | translate <command> ...]");
            exit(1);
        }
    }
}

/// Load the user's config, detect their shell, and print init code.
fn cmd_init() {
    let config = match config::SemanticConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {e}");
            eprintln!("Run `semantic` (no args) to set up your config first.");
            exit(1);
        }
    };

    let detected_shell = shell::detect_shell();

    // use the configured default shell if set, otherwise use the detected one
    let shell = if config.shells.default.is_empty() {
        &detected_shell
    } else {
        &config.shells.default
    };

    let output = shell::generate_init(&config.commands, &config.paths, shell);
    print!("{output}");
}

/// Look up a semantic command in config and execute the real command.
/// Called as: semantic translate <semantic_cmd> [args...]
fn cmd_translate(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: semantic translate <command> [args...]");
        exit(1);
    }

    let config = match config::SemanticConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {e}");
            exit(1);
        }
    };

    let semantic_cmd = &args[0];
    let extra_args = &args[1..];

    // look up the semantic command in the config
    let real_cmd = match config.commands.get(semantic_cmd.as_str()) {
        Some(cmd) => cmd,
        None => {
            eprintln!("Unknown semantic command: {semantic_cmd}");
            exit(1);
        }
    };

    // the real command might have multiple parts (e.g. "sudo pacman -S")
    let parts: Vec<&str> = real_cmd.split_whitespace().collect();
    let (program, builtin_args) = parts.split_first().expect("empty command mapping");

    // translate any path arguments (e.g. /apps -> /usr/bin)
    let translated_args: Vec<String> = extra_args
        .iter()
        .map(|arg| {
            config.paths.get(arg.as_str())
                .cloned()
                .unwrap_or_else(|| arg.clone())
        })
        .collect();

    // combine: program + builtin args from mapping + user's extra args
    let status = Command::new(program)
        .args(builtin_args)
        .args(&translated_args)
        .status();

    match status {
        Ok(s) => exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("Failed to run `{real_cmd}`: {e}");
            exit(1);
        }
    }
}
