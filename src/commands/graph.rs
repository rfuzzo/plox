use std::process::ExitCode;
use std::{env, path::PathBuf};

use log::{error, info};
use petgraph::dot::{Config, Dot};

use crate::*;

pub fn graph(
    game: ESupportedGame,
    game_folder: &Option<PathBuf>,
    rules_path: &Option<String>,
    mod_list: &Option<PathBuf>,
    config: Option<PathBuf>,
) -> ExitCode {
    // get game root
    let root = match game_folder {
        Some(path) => path.clone(),
        None => env::current_dir().expect("No current working dir"),
    };

    // get default rules dir
    let rules_dir = if let Some(path) = rules_path {
        PathBuf::from(path)
    } else {
        get_default_rules_dir(game)
    };

    let game_version = get_game_version(game);

    // gather mods (optionally from a list)
    let mods: Vec<PluginData>;
    if let Some(modlist_path) = mod_list {
        mods = read_file_as_list(modlist_path, &game_version);
    } else {
        mods = match game {
            ESupportedGame::Morrowind => gather_tes3_mods(&root),
            ESupportedGame::Cyberpunk => gather_cp77_mods(&root, &game_version),
            ESupportedGame::Openmw => gather_openmw_mods(&config),
        };
        if mods.is_empty() {
            info!("No mods found");
            return ExitCode::FAILURE;
        }
    }

    let mut parser = parser::get_parser(game, game_version);
    if let Err(e) = parser.parse(rules_dir) {
        error!("Parser init failed: {}", e);
        return ExitCode::FAILURE;
    }

    let data = sorter::get_graph_data(&mods, &parser.order_rules, &parser.warning_rules);
    let g = sorter::build_graph(&data);

    {
        let viz = Dot::with_config(&g, &[Config::EdgeNoLabel]);
        // write to file
        let mut file = std::fs::File::create(GRAPH_FILE).expect("file create failed");
        std::io::Write::write_all(&mut file, format!("{:?}", viz).as_bytes())
            .expect("write failed");
    }

    ExitCode::SUCCESS
}
