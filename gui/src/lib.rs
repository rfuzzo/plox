#![warn(clippy::all, rust_2018_idioms)]

mod app;

use std::{env, path::PathBuf, sync::mpsc::Sender};

pub use app::TemplateApp;
use log::error;
use plox::{
    detect_game, download_latest_rules, gather_mods, get_default_rules_dir,
    parser::{self, Warning},
    sorter::new_stable_sorter,
};

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
enum ETheme {
    Dark,
    Light,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
struct AppSettings {
    /// Specifies an openmw config file to use
    config: Option<PathBuf>,

    /// Specifies the game to use
    game: Option<plox::ESupportedGame>,

    /// set to not download rules
    no_rules_download: bool,
}
impl AppSettings {
    fn from_file(arg: &str) -> Self {
        // deserialize from toml file
        match std::fs::read_to_string(arg) {
            Ok(s) => match toml::from_str(&s) {
                Ok(s) => s,
                Err(e) => {
                    error!("Error deserializing settings: {}", e);
                    AppSettings::default()
                }
            },
            Err(e) => {
                error!("Error reading settings file: {}", e);
                AppSettings::default()
            }
        }
    }
}

#[derive(Debug, Clone)]
struct AppData {
    game: plox::ESupportedGame,
    old_order: Vec<String>,
    new_order: Vec<String>,
    warnings: Vec<Warning>,
    plugin_warning_map: Vec<(String, usize)>,
}

fn init_parser(settings: AppSettings, tx: Sender<String>) -> Option<AppData> {
    // game
    let game = if let Some(game) = settings.game {
        let _ = tx.send(format!("Using game: {:?}", game));
        game
    } else {
        match detect_game() {
            Some(g) => {
                let _ = tx.send(format!("Detected game: {:?}", g));
                g
            }
            None => {
                let _ = tx.send("No game detected".to_string());
                return None;
            }
        }
    };

    let root = env::current_dir().expect("No current working dir");

    // rules
    let rules_dir = get_default_rules_dir(game);
    if !settings.no_rules_download {
        let _ = tx.send("Downloading rules".to_string());
        download_latest_rules(game, &rules_dir);
    } else {
        let _ = tx.send("Skipping rules download".to_string());
    }

    // mods
    let _ = tx.send("Gathering mods".to_string());
    let mods = gather_mods(&root, game, settings.config);

    // parser
    let mut parser = parser::get_parser(game);
    let _ = tx.send("Initializing parser".to_string());
    if let Err(e) = parser.init(rules_dir) {
        error!("Parser init failed: {}", e);
        let _ = tx.send(format!("Parser init failed: {}", e));
        return None;
    }

    // evaluate
    let _ = tx.send("Evaluating plugins".to_string());
    parser.evaluate_plugins(&mods);
    let warnings = parser.warnings.clone();
    let mut plugin_warning_map = vec![];
    for (i, w) in warnings.iter().enumerate() {
        for p in &w.get_plugins() {
            plugin_warning_map.push((p.clone(), i));
        }
    }

    // sort
    //let mut new_order = mods.clone();
    // check order first
    //match check_order(&mods, &parser.order_rules) {
    //    true => {
    //        // exit
    //        info!("Mods are in correct order, no sorting needed.");
    //        let _ = tx.send("Mods are in correct order, no sorting needed.".to_string());
    //     }
    //    false => {
    let mut sorter = new_stable_sorter();
    let _ = tx.send("Sorting mods".to_string());
    let new_order = match sorter.topo_sort(game, &mods, &parser.order_rules) {
        Ok(new) => new,
        Err(e) => {
            error!("error sorting: {e:?}");
            let _ = tx.send(format!("error sorting: {e:?}"));
            return None;
        }
    };
    //}
    //}

    let r = AppData {
        game,
        old_order: mods,
        new_order,
        warnings,
        plugin_warning_map,
    };

    Some(r)
}
