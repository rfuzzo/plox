#![warn(clippy::all, rust_2018_idioms)]

mod app;

use std::env;

pub use app::TemplateApp;
use log::error;
use plox::{
    download_latest_rules, gather_mods, get_default_rules_dir,
    parser::{self, Parser},
    sorter::new_stable_sorter,
};

type ParserResult = (Parser, Vec<String>, Vec<(String, usize)>);

fn init_parser(game: plox::ESupportedGame, tx: std::sync::mpsc::Sender<Option<ParserResult>>) {
    // TODO this blocks UI and sorts everything
    // TODO run a terminal?
    let root = env::current_dir().expect("No current working dir");

    // rules
    let rules_dir = get_default_rules_dir(game);
    download_latest_rules(game, &rules_dir);

    // mods
    let mods = gather_mods(&root, game);

    // parser
    let mut parser = parser::get_parser(game);
    if let Err(e) = parser.init(rules_dir) {
        error!("Parser init failed: {}", e);
        todo!("Handle error");
        //self.warning = format!("Parser init failed: {}", e);
    }

    // evaluate
    parser.evaluate_plugins(&mods);
    let warnings = parser.warnings.clone();
    let new_order;
    let mut plugin_warning_map = vec![];

    for (i, w) in warnings.iter().enumerate() {
        for p in &w.get_plugins() {
            plugin_warning_map.push((p.clone(), i));
        }
    }

    // sort
    let mut sorter = new_stable_sorter();
    match sorter.topo_sort(&mods, &parser.order_rules) {
        Ok(new) => {
            new_order = new;
        }
        Err(e) => {
            error!("error sorting: {e:?}");
            //return None;
            //self.warning = format!("error sorting: {e:?}");
            todo!("Handle error");
        }
    }

    //self.parser = Some(parser);
    let r: Option<ParserResult> = Some((parser, new_order, plugin_warning_map));
    let _ = tx.send(r);
}
