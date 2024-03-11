#![warn(clippy::all, rust_2018_idioms)]

mod app;

use std::{env, sync::mpsc::Sender};

pub use app::TemplateApp;
use log::error;
use plox::{
    download_latest_rules, gather_mods, get_default_rules_dir,
    parser::{self, Warning},
    sorter::new_stable_sorter,
};

#[derive(Debug, Clone)]
struct AppData {
    game: plox::ESupportedGame,
    new_order: Vec<String>,
    warnings: Vec<Warning>,
    plugin_warning_map: Vec<(String, usize)>,
}

fn init_parser(game: plox::ESupportedGame, tx: Sender<String>) -> Option<AppData> {
    let root = env::current_dir().expect("No current working dir");

    // rules
    let _ = tx.send("Downloading rules".to_string());
    let rules_dir = get_default_rules_dir(game);
    download_latest_rules(game, &rules_dir);

    // mods
    let _ = tx.send("Gathering mods".to_string());
    let mods = gather_mods(&root, game);

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
    let mut sorter = new_stable_sorter();
    let _ = tx.send("Sorting mods".to_string());
    let new_order = match sorter.topo_sort(&mods, &parser.order_rules) {
        Ok(new) => new,
        Err(e) => {
            error!("error sorting: {e:?}");
            let _ = tx.send(format!("error sorting: {e:?}"));
            return None;
        }
    };

    let r = AppData {
        game,
        new_order,
        warnings,
        plugin_warning_map,
    };

    Some(r)
}
