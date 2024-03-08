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
    } else if is_current_directory_name("Cyberpunk 2077") {
        info!("Detected game: {:?}", ESupportedGame::Cyberpunk);
        ESupportedGame::Cyberpunk
    } else if is_current_directory_name("Morrowind") {
        info!("Detected game: {:?}", ESupportedGame::Morrowind);
        ESupportedGame::Morrowind
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
        println!("\nPress any button to continue");
        let mut buffer = String::new();
        let _ = std::io::stdin().read_line(&mut buffer);
    }

    code
}
