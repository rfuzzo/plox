use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::{error, info};

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
    /// Outputs the rules as a graphviz dot file
    Graph {
        /// Root game folder (e.g. "Cyberpunk 2077" or "Morrowind"). Default is current working directory
        #[arg(short, long)]
        game_folder: Option<PathBuf>,

        /// Folder to read sorting rules from. Default is ./mlox for TES3
        #[arg(short, long)]
        rules_dir: Option<String>,

        /// Read the input mods from a file instead of checking the root folder
        #[arg(short, long)]
        mod_list: Option<PathBuf>,

        /// (OpenMW only) Path to the openmw.cfg file
        #[arg(short, long)]
        config: Option<PathBuf>,
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
        Command::Graph {
            game_folder,
            rules_dir,
            mod_list,
            config,
        } => graph(game, game_folder, rules_dir, mod_list, config.clone()),
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
