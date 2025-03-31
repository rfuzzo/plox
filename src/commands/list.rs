use std::path::PathBuf;
use std::process::ExitCode;

use log::info;

use crate::*;

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

    for m in gather_mods(&root, game, &None, config) {
        println!("{}", m.name);
        //info!("{}", m);
    }

    ExitCode::SUCCESS
}
