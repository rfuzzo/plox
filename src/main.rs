use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::{error, info, warn};

use plox::rules::TRule;
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

        /// Disable automatic downloading of latest ruleset
        #[arg(short, long)]
        no_download: bool,

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

fn main() -> ExitCode {
    let cli = Cli::parse();

    // logger
    let mut level = ELogLevel::Info;
    if let Some(lvl) = cli.log_level {
        level = lvl;
    }
    let env = Env::default()
        .default_filter_or(log_level_to_str(level))
        .default_write_style_or("always");
    env_logger::Builder::from_env(env).init();

    // detect game
    let game = if let Some(game) = cli.game {
        game
    } else if is_current_directory_name("Cyberpunk 2077") {
        ESupportedGame::Cyberpunk
    } else if is_current_directory_name("Morrowind") {
        ESupportedGame::Morrowind
    } else {
        error!("No game specified or detected");
        return ExitCode::FAILURE;
    };

    info!("Detected game: {:?}", game);

    let code = match &cli.command {
        Command::List { root } => list_mods(root, game),
        Command::Verify { rules_dir } => verify(game, rules_dir),
        Command::Sort {
            game_folder: root,
            rules_dir,
            mod_list,
            dry_run,
            unstable,
            no_download,
        } => sort(
            game,
            root,
            rules_dir,
            mod_list,
            *dry_run,
            *unstable,
            *no_download,
        ),
    };

    if !cli.non_interactive {
        println!("Press any button to continue");
        let mut buffer = String::new();
        let _ = std::io::stdin().read_line(&mut buffer);
    }

    code
}

/// Sorts the current mod load order according to specified rules
fn sort(
    game: ESupportedGame,
    root: &Option<PathBuf>,
    rules_path: &Option<String>,
    mod_list: &Option<PathBuf>,
    dry_run: bool,
    unstable: bool,
    no_download: bool,
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
        get_default_rules_dir(game)
    };

    // gather mods (optionally from a list)
    let mods: Vec<String>;
    if let Some(modlist_path) = mod_list {
        mods = read_file_as_list(modlist_path);
    } else {
        mods = gather_mods(&root, game);
        if mods.is_empty() {
            info!("No mods found");
            return ExitCode::FAILURE;
        }
    }

    if !no_download {
        download_latest_rules(game, &rules_dir);
    }

    let mut parser = parser::get_parser(game);
    parser.init(rules_dir);

    if parser.rules.is_empty() {
        warn!("No rules found to evaluate");
        return ExitCode::FAILURE;
    }

    // Print Warnings and Notes
    info!("Evaluating mod list...");
    for rule in &parser.rules {
        if rule.eval(&mods) {
            match rule {
                rules::Rule::Order(_) => {
                    // Order rules don't get evaluated
                }
                rules::Rule::Note(n) => {
                    info!("[NOTE]\n{}\n", n.get_comment());
                }
                rules::Rule::Conflict(c) => {
                    warn!("[CONFLICT]\n{}\n", c.get_comment());
                }
                rules::Rule::Requires(r) => {
                    warn!("[REQUIRES]\n{}\n", r.get_comment());
                }
                rules::Rule::Patch(p) => {
                    warn!("[Patch]\n{}\n", p.get_comment());
                }
            }
        }
    }

    // Sort
    let order_rules = get_order_rules(&parser.rules);
    if order_rules.is_empty() {
        info!("No order rules found, nothing to sort");
        return ExitCode::SUCCESS;
    }

    info!("Sorting mods...");
    let mut sorter = if unstable {
        sorter::new_unstable_sorter()
    } else {
        sorter::new_stable_sorter()
    };
    match sorter.topo_sort(&mods, &order_rules) {
        Ok(result) => {
            if dry_run {
                info!("Dry run...");
                info!("New:\n{:?}", result);
            } else {
                info!("Current:\n{:?}", &mods);

                if mods.eq(&result) {
                    info!("Mods are in correct order, no sorting needed.");
                } else {
                    info!("New:\n{:?}", result);

                    update_new_load_order(game, result);
                }
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            error!("error sorting: {e:?}");
            ExitCode::FAILURE
        }
    }
}

/// Verifies integrity of the specified rules
fn verify(game: ESupportedGame, rules_path: &Option<String>) -> ExitCode {
    let rules_dir = if let Some(path) = rules_path {
        PathBuf::from(path)
    } else {
        get_default_rules_dir(game)
    };

    info!("Verifying rules from {} ...", rules_dir.display());

    let mut parser = parser::get_parser(game);
    parser.init(rules_dir);

    if parser.rules.is_empty() {
        warn!("No rules found to evaluate");
        return ExitCode::FAILURE;
    }

    let order = get_order_rules(&parser.rules);
    let mods = debug_get_mods_from_rules(&order);
    match sorter::new_unstable_sorter().topo_sort(&mods, &order) {
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
fn list_mods(root: &Option<PathBuf>, game: ESupportedGame) -> ExitCode {
    info!("Printing active mods...");

    let root = match root {
        Some(path) => path.clone(),
        None => env::current_dir().expect("No current working dir"),
    };

    for m in gather_mods(&root, game) {
        println!("{}", m);
        //info!("{}", m);
    }

    ExitCode::SUCCESS
}
