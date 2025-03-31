#![warn(clippy::all, rust_2018_idioms)]

mod app;

use std::{
    env,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

pub use app::TemplateApp;
use log::{error, warn};
use plox::{
    conflict2, detect_game, download_latest_rules, gather_mods, get_default_rules_dir,
    get_game_version,
    parser::{self, Warning},
    sorter::new_stable_sorter,
};

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const PLOX_LOG_FILE: &str = "plox.log";
pub const PLOX_CONF_FILE: &str = "plox.toml";

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
struct AppSettings {
    /// Specifies an openmw config file to use
    config: Option<PathBuf>,

    /// Specifies the game to use
    game: Option<plox::ESupportedGame>,

    /// set to not download rules
    no_rules_download: bool,

    /// log level
    log_level: Option<String>,

    /// use a log file bool
    log_to_file: bool,

    /// ignore warnings
    ignore_warnings: bool,
}
impl AppSettings {
    fn from_file(arg: &Path) -> Self {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ELoadStatus {
    Conflicts,
    Cycle,
    Success,
}

#[derive(Debug, Clone)]
struct AppData {
    game: plox::ESupportedGame,
    old_order: Vec<String>,
    new_order: Vec<String>,
    warnings: Vec<Warning>,
    plugin_warning_map: Vec<(String, usize)>,
    status: ELoadStatus,
    game_version: Option<String>,
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
    let game_version = get_game_version(game);

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
    let mods = gather_mods(&root, game, &game_version, settings.config);

    // parser
    let mut parser = parser::get_parser(game, game_version.clone());
    let _ = tx.send("Initializing parser".to_string());
    if let Err(e) = parser.parse(rules_dir) {
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

    // check if there are any conflicts
    let mut has_conflicts = false;
    if !warnings.is_empty() {
        // check if there are any conflict rules in the warnings
        // conflict2

        for w in &warnings {
            if conflict2(&w.rule).is_some() {
                has_conflicts = true;
                warn!("Conflict detected: {:?}", w);
            }
        }
    }

    // ignore warnings
    if settings.ignore_warnings {
        has_conflicts = false;
    }

    let status;
    // sort
    let mut new_order = mods.iter().map(|m| m.name.clone()).collect();
    if !has_conflicts {
        if !&parser.order_rules.is_empty() {
            let mut sorter = new_stable_sorter();
            let _ = tx.send("Sorting mods".to_string());

            match sorter.topo_sort(game, &mods, &parser.order_rules, &parser.warning_rules) {
                Ok(new) => {
                    new_order = new;
                    status = ELoadStatus::Success;
                }
                Err(e) => {
                    let error_msg = format!("{e:?}");
                    error!("error sorting: {error_msg}");

                    // TODO better
                    if error_msg.contains("Out of iterations") {
                        let _ = tx.send("Cycle detected, skipping sort.".to_string());
                        status = ELoadStatus::Cycle;
                    } else {
                        let _ = tx.send(format!("error sorting: {e:?}"));
                        // exit
                        return None;
                    }
                }
            };
        } else {
            status = ELoadStatus::Success;
        }
    } else {
        warn!("Conflicts detected, skipping sort");
        let _ = tx.send("Conflicts detected, skipping sort".to_string());
        status = ELoadStatus::Conflicts;
    }

    let r = AppData {
        game,
        old_order: mods.iter().map(|m| m.name.clone()).collect(),
        new_order,
        warnings,
        plugin_warning_map,
        status,
        game_version,
    };

    Some(r)
}
