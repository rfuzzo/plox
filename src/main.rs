use std::process::ExitCode;
use std::{env, path::PathBuf};

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::{debug, error, info, warn};

use plox::rules::{EWarningRule, TWarningRule};
use plox::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Set the log level, default is "info"
    #[arg(short, long)]
    log_level: Option<ELogLevel>,

    /// Set the game to evaluate, if no game is specified it will attempt to deduce the game from the current working directory
    #[arg(short, long)]
    game: Option<ESupportedGame>,

    /// Disable user input
    #[arg(short, long)]
    non_interactive: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone, Debug)]
enum Command {
    /// Sorts the current mod load order according to specified rules
    Sort {
        /// Root game folder (e.g. "Cyberpunk 2077" or "Morrowind"). Default is current working directory
        #[arg(short, long)]
        game_folder: Option<PathBuf>,

        /// Folder to read sorting rules from. Default is ./mlox for TES3
        #[arg(short, long)]
        rules_dir: Option<String>,

        /// Just print the suggested load order without sorting
        #[arg(short, long)]
        dry_run: bool,

        /// Use the potentially faster unstable sorter
        #[arg(short, long)]
        unstable: bool,

        /// Disable automatic downloading of latest ruleset
        #[arg(short, long)]
        no_download: bool,

        /// Read the input mods from a file instead of checking the root folder
        #[arg(short, long)]
        mod_list: Option<PathBuf>,

        /// (OpenMW only) Path to the openmw.cfg file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Lists the current mod load order
    List {
        /// Root game folder (e.g. "Cyberpunk 2077" or "Morrowind"). Default is current working directory
        #[arg(short, long)]
        root: Option<PathBuf>,

        /// (OpenMW only) Path to the openmw.cfg file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Verifies integrity of the specified rules
    Verify {
        /// Folder to read sorting rules from. Default is ./plox or ./mlox for TES3
        #[arg(short, long)]
        rules_dir: Option<String>,
    },
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("{}", e);

            println!("\nPress any button to continue");
            let mut buffer = String::new();
            let _ = std::io::stdin().read_line(&mut buffer);

            return ExitCode::FAILURE;
        }
    };

    // logger
    let mut level = ELogLevel::Info;
    if let Some(lvl) = cli.log_level {
        level = lvl;
    }
    let env = Env::default()
        .default_filter_or(log_level_to_str(level))
        .default_write_style_or("always");
    env_logger::Builder::from_env(env)
        .format_timestamp(None)
        .init();

    // detect game
    let game = if let Some(game) = cli.game {
        info!("Set game to: {:?}", game);
        game
    } else if let Some(g) = detect_game() {
        info!("Detected game: {:?}", g);
        g
    } else {
        error!("No game specified or detected");
        if !cli.non_interactive {
            println!("Press any button to continue");
            let mut buffer = String::new();
            let _ = std::io::stdin().read_line(&mut buffer);
        }
        return ExitCode::FAILURE;
    };

    let code = match &cli.command {
        Command::List { root, config } => list_mods(root, game, config.clone()),
        Command::Verify { rules_dir } => verify(game, rules_dir),
        Command::Sort {
            game_folder: root,
            rules_dir,
            mod_list,
            dry_run,
            unstable,
            no_download,
            config,
        } => sort(CliSortOptions {
            game,
            game_folder: root.clone(),
            rules_dir: rules_dir.clone(),
            mod_list: mod_list.clone(),
            dry_run: *dry_run,
            unstable: *unstable,
            no_download: *no_download,
            config: config.clone(),
        }),
    };

    if !cli.non_interactive {
        println!("\nPress any button to continue");
        let mut buffer = String::new();
        let _ = std::io::stdin().read_line(&mut buffer);
    }

    code
}

pub struct CliSortOptions {
    pub game: ESupportedGame,
    pub game_folder: Option<PathBuf>,
    pub rules_dir: Option<String>,
    pub mod_list: Option<PathBuf>,
    pub dry_run: bool,
    pub unstable: bool,
    pub no_download: bool,
    pub config: Option<PathBuf>,
}

/// Sorts the current mod load order according to specified rules
pub fn sort(options: CliSortOptions) -> ExitCode {
    let game = options.game;
    let root = options.game_folder;
    let rules_path = options.rules_dir;
    let mod_list = options.mod_list;
    let dry_run = options.dry_run;
    let unstable = options.unstable;
    let no_download = options.no_download;
    let config = options.config;

    // get game root
    let root = match root {
        Some(path) => path.clone(),
        None => env::current_dir().expect("No current working dir"),
    };

    // get default rules dir
    let rules_dir = if let Some(path) = rules_path {
        PathBuf::from(path)
    } else {
        get_default_rules_dir(game)
    };

    if !no_download {
        download_latest_rules(game, &rules_dir);
    } else {
        info!("Skipping downloading latest rules")
    }

    // gather mods (optionally from a list)
    let mods: Vec<PluginData>;
    if let Some(modlist_path) = mod_list {
        mods = read_plugin_data(modlist_path);
    } else {
        mods = match game {
            ESupportedGame::Morrowind => gather_tes3_mods(&root),
            ESupportedGame::Cyberpunk => gather_cp77_mods(&root),
            ESupportedGame::Openmw => gather_openmw_mods(&config),
        };
        if mods.is_empty() {
            info!("No mods found");
            return ExitCode::FAILURE;
        }
    }

    let mut parser = parser::get_parser(game);
    if let Err(e) = parser.init(rules_dir) {
        error!("Parser init failed: {}", e);
        return ExitCode::FAILURE;
    }

    // Print Warnings and Notes
    if parser.warning_rules.is_empty() {
        warn!("No rules found to evaluate");
    } else {
        info!("Evaluating mod list...\n");
        debug!("{:?}", &mods);

        parser.evaluate_plugins(&mods);
        for warning in parser.warnings {
            let rule = warning.rule;
            match rule {
                EWarningRule::Note(n) => {
                    info!("[NOTE]\n{}", n.get_comment());
                    info!("Reference: [{}]", n.plugins.join(";"));
                }
                EWarningRule::Conflict(c) => {
                    warn!("[CONFLICT]\n{}", c.get_comment());
                    info!("Reference: [{}]", c.plugins.join(";"));
                }
                EWarningRule::Requires(r) => {
                    error!("[REQUIRES]\n{}", r.get_comment());
                    info!("Reference: [{}]", r.plugins.join(";"));
                }
                EWarningRule::Patch(p) => {
                    warn!("[Patch]\n{}", p.get_comment());
                    info!("Reference: [{}]", p.plugins.join(";"));
                }
            }
            println!();
        }
    }

    // Sort
    if parser.order_rules.is_empty() {
        warn!("No rules found to sort");
        ExitCode::SUCCESS
    } else {
        info!("Sorting mods...");
        let mut sorter = if unstable {
            sorter::new_unstable_sorter()
        } else {
            sorter::new_stable_sorter()
        };

        // check order first
        // match check_order(&mods, &parser.order_rules) {
        //     true => {
        //         // exit
        //         info!("Mods are in correct order, no sorting needed.");
        //         return ExitCode::SUCCESS;
        //     }
        //     false => {}
        // }

        match sorter.topo_sort(game, &mods, &parser.order_rules) {
            Ok(result) => {
                if dry_run {
                    info!("Dry run...");

                    debug!("Old:\n{:?}", &mods);
                    debug!("New:\n{:?}", result);

                    if mods
                        .iter()
                        .map(|f| f.name.clone())
                        .collect::<Vec<_>>()
                        .eq(&result)
                    {
                        info!("Mods are in correct order, no sorting needed.");
                    } else {
                        info!("New order:\n{:?}", result);
                    }

                    ExitCode::SUCCESS
                } else {
                    info!("Current:\n{:?}", &mods);

                    if mods
                        .iter()
                        .map(|f| f.name.clone())
                        .collect::<Vec<_>>()
                        .eq(&result)
                    {
                        info!("Mods are in correct order, no sorting needed.");
                        ExitCode::SUCCESS
                    } else {
                        info!("New:\n{:?}", result);

                        match update_new_load_order(game, &result, config) {
                            Ok(_) => {
                                info!("Update successful");
                                ExitCode::SUCCESS
                            }
                            Err(e) => {
                                error!("Could not updae load order: {}", e);
                                ExitCode::FAILURE
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("error sorting: {e:?}");
                ExitCode::FAILURE
            }
        }
    }
}

/// Verifies integrity of the specified rules
pub fn verify(game: ESupportedGame, rules_path: &Option<String>) -> ExitCode {
    let rules_dir = if let Some(path) = rules_path {
        PathBuf::from(path)
    } else {
        get_default_rules_dir(game)
    };

    info!("Verifying rules from {} ...", rules_dir.display());

    let mut parser = parser::get_parser(game);
    if let Err(e) = parser.init(rules_dir) {
        error!("Parser init failed: {}", e);
        return ExitCode::FAILURE;
    }

    if parser.warning_rules.is_empty() {
        warn!("No rules found to evaluate");
        return ExitCode::FAILURE;
    }

    let mods = debug_get_mods_from_order_rules(&parser.order_rules);
    match sorter::new_unstable_sorter().topo_sort(game, &mods, &parser.order_rules) {
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

/// Lists the current mod load order
pub fn list_mods(
    root: &Option<PathBuf>,
    game: ESupportedGame,
    config: Option<PathBuf>,
) -> ExitCode {
    info!("Printing active mods...");

    let root = match root {
        Some(path) => path.clone(),
        None => env::current_dir().expect("No current working dir"),
    };

    for m in gather_mods(&root, game, config) {
        println!("{}", m.name);
        //info!("{}", m);
    }

    ExitCode::SUCCESS
}
