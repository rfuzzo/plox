use clap::{Parser, Subcommand};
use log::{error, info, warn};
use plox::rules::TRule;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use plox::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Set game
    #[arg(short, long)]
    game: Option<ESupportedGame>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Clone, Debug)]
enum Command {
    /// Sorts the current mod load order according to specified rules
    Sort {
        /// Root game folder (e.g. "Cyberpunk 2077" or "Data Files"). Default is current working directory
        #[arg(short, long)]
        game_folder: Option<PathBuf>,

        /// Folder to read sorting rules from. Default is ./plox or ./mlox for TES3
        #[arg(short, long)]
        rules_dir: Option<String>,

        /// Just print the suggested load order without sorting
        #[arg(short, long)]
        dry_run: bool,

        /// Use the potentially faster unstable sorter
        #[arg(short, long)]
        unstable: bool,

        /// Read the input mods from a file instead of checking the root folder
        #[arg(short, long)]
        mod_list: Option<PathBuf>,
    },
    /// Lists the current mod load order
    List {
        /// Root game folder (e.g. "Cyberpunk 2077" or "Data Files"). Default is current working directory
        #[arg(short, long)]
        root: Option<PathBuf>,
    },
    /// Verifies integrity of the specified rules
    Verify {
        /// Folder to read sorting rules from. Default is ./plox or ./mlox for TES3
        #[arg(short, long)]
        rules_dir: Option<String>,
    },
}

fn is_current_directory_name(name_to_check: &str) -> bool {
    // Get the current directory
    if let Ok(current_dir) = env::current_dir() {
        // Extract the directory name
        if let Some(dir_name) = current_dir.file_name() {
            // Convert the directory name to a string
            if let Some(name) = dir_name.to_str() {
                // Check if the directory name is "Cyberpunk"
                return name == name_to_check;
            }
        }
    }

    false
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // TODO logging
    const CARGO_NAME: &str = env!("CARGO_PKG_NAME");
    let _ = simple_logging::log_to_file(format!("{}.log", CARGO_NAME), log::LevelFilter::Debug);
    //simple_logging::log_to_stderr(log::LevelFilter::Info);

    // detect game
    let game = if let Some(game) = cli.game {
        game
    } else if is_current_directory_name("Cyberpunk 2077") {
        ESupportedGame::Cyberpunk
    } else if is_current_directory_name("Data Files") {
        // TODO support root tes3 dir
        // || is_current_directory_name("Morrowind")
        ESupportedGame::Morrowind
    } else {
        error!("No game specified to verify");
        return ExitCode::FAILURE;
    };

    match &cli.command {
        Some(Command::List { root }) => list_mods(root, game),
        Some(Command::Verify { rules_dir }) => verify(game, rules_dir),
        Some(Command::Sort {
            game_folder: root,
            rules_dir,
            mod_list,
            dry_run,
            unstable,
        }) => sort(game, root, rules_dir, mod_list, *dry_run, *unstable),
        None => ExitCode::FAILURE,
    }
}

/// Sorts the current mod load order according to specified rules
fn sort(
    game: ESupportedGame,
    root: &Option<PathBuf>,
    rules_path: &Option<String>,
    mod_list: &Option<PathBuf>,
    dry_run: bool,
    unstable: bool,
) -> ExitCode {
    info!("Sorting mods...");

    // get game root
    let root = match root {
        Some(path) => path.clone(),
        None => env::current_dir().expect("No current working dir"),
    };

    // get default rules dir
    let rules_dir = if let Some(path) = rules_path {
        PathBuf::from(path)
    } else {
        match get_default_rules_dir(game) {
            Ok(value) => value,
            Err(value) => return value,
        }
    };

    // gather mods (optionally from a list)
    let mods: Vec<String>;
    if let Some(modlist_path) = mod_list {
        mods = read_file_as_list(modlist_path);
    } else {
        match gather_mods(&root, game) {
            Ok(m) => mods = m,
            Err(e) => {
                info!("No mods found: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    let mut parser = parser::get_parser(game);
    match parser.parse(rules_dir) {
        Ok(rules) => {
            // Print Warnings
            for rule in &rules {
                if rule.eval(&mods) {
                    match rule {
                        rules::Rule::Order(_) => {}
                        rules::Rule::Note(n) => {
                            println!("[NOTE]\n{}\n", n.get_comment());
                            info!("[NOTE]\n{}\n", n.get_comment());
                        }
                        rules::Rule::Conflict(c) => {
                            println!("[CONFLICT]\n{}\n", c.get_comment());
                            warn!("[CONFLICT]\n{}\n", c.get_comment());
                        }
                        rules::Rule::Requires(r) => {
                            println!("[REQUIRES]\n{}\n", r.get_comment());
                            warn!("[REQUIRES]\n{}\n", r.get_comment());
                        }
                    }
                }
            }

            // Sort
            let mut sorter = if unstable {
                Sorter::new_unstable()
            } else {
                Sorter::new_stable()
            };

            let order_rules = get_order_rules(&rules);
            if !order_rules.is_empty() {
                match sorter.topo_sort(&mods, &order_rules) {
                    Ok(result) => {
                        if dry_run {
                            info!("Dry run...");
                            info!("{result:?}");
                        } else {
                            info!("Sorting mods...");
                            info!("{:?}", &mods);
                            info!("New:");
                            info!("{result:?}");

                            // TODO check if nothing to sort

                            // TODO update on disk
                        }

                        return ExitCode::SUCCESS;
                    }
                    Err(e) => {
                        error!("error sorting: {e:?}");
                        return ExitCode::FAILURE;
                    }
                }
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            error!("error parsing file: {e:?}");
            ExitCode::FAILURE
        }
    }
}

/// Verifies integrity of the specified rules
fn verify(game: ESupportedGame, rules_path: &Option<String>) -> ExitCode {
    let rules_dir = if let Some(path) = rules_path {
        PathBuf::from(path)
    } else {
        match get_default_rules_dir(game) {
            Ok(value) => value,
            Err(value) => return value,
        }
    };

    info!("Verifying rules from {} ...", rules_dir.display());

    let mut parser = parser::get_parser(game);
    match parser.parse(rules_dir) {
        Ok(rules) => {
            let order = get_order_rules(&rules);
            let mods = debug_get_mods_from_rules(&order);
            match Sorter::new_unstable().topo_sort(&mods, &order) {
                Ok(_) => {
                    info!("Verify SUCCESS");
                    ExitCode::SUCCESS
                }
                Err(_) => {
                    error!("Verify FAILURE");
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            error!("error parsing file: {e:?}");
            ExitCode::FAILURE
        }
    }
}

/// Lists the current mod load order
fn list_mods(root: &Option<PathBuf>, game: ESupportedGame) -> ExitCode {
    info!("Printing active mods...");

    let root = match root {
        Some(path) => path.clone(),
        None => env::current_dir().expect("No current working dir"),
    };

    match gather_mods(&root, game) {
        Ok(mods) => {
            for m in mods {
                println!("{}", m);
                info!("{}", m);
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            error!("{}", e);
            ExitCode::FAILURE
        }
    }
}
