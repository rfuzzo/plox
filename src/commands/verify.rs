use std::path::PathBuf;
use std::process::ExitCode;

use log::{error, info, warn};

use crate::*;

/// Verifies integrity of the specified rules
pub fn verify(game: ESupportedGame, rules_path: &Option<String>) -> ExitCode {
    let rules_dir = if let Some(path) = rules_path {
        PathBuf::from(path)
    } else {
        get_default_rules_dir(game)
    };

    info!("Verifying rules from {} ...", rules_dir.display());

    let game_version = get_game_version(game);
    let mut parser = parser::get_parser(game, game_version);
    if let Err(e) = parser.parse(rules_dir) {
        error!("Parser init failed: {}", e);
        return ExitCode::FAILURE;
    }

    if parser.warning_rules.is_empty() {
        warn!("No rules found to evaluate");
        return ExitCode::FAILURE;
    }

    let mods = debug_get_mods_from_order_rules(&parser.order_rules);
    match sorter::new_unstable_sorter().topo_sort(
        game,
        &mods,
        &parser.order_rules,
        &parser.warning_rules,
    ) {
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
