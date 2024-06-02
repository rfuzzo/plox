use std::process::ExitCode;
use std::{env, path::PathBuf};

use log::{debug, error, info, warn};

use crate::*;

pub struct CliSortOptions {
    pub game: ESupportedGame,
    pub game_folder: Option<PathBuf>,
    pub rules_dir: Option<String>,
    pub mod_list: Option<PathBuf>,
    pub dry_run: bool,
    pub unstable: bool,
    pub no_download: bool,
    pub config: Option<PathBuf>,
}

/// Sorts the current mod load order according to specified rules
pub fn sort(options: CliSortOptions) -> ExitCode {
    let game = options.game;
    let root = options.game_folder;
    let rules_path = options.rules_dir;
    let mod_list = options.mod_list;
    let dry_run = options.dry_run;
    let unstable = options.unstable;
    let no_download = options.no_download;
    let config = options.config;

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

    if !no_download {
        download_latest_rules(game, &rules_dir);
    } else {
        info!("Skipping downloading latest rules")
    }

    // gather mods (optionally from a list)
    let mods: Vec<PluginData>;
    if let Some(modlist_path) = mod_list {
        mods = read_file_as_list(modlist_path);
    } else {
        mods = match game {
            ESupportedGame::Morrowind => gather_tes3_mods(&root),
            ESupportedGame::Cyberpunk => gather_cp77_mods(&root),
            ESupportedGame::Openmw => gather_openmw_mods(&config),
        };
        if mods.is_empty() {
            info!("No mods found");
            return ExitCode::FAILURE;
        }
    }

    let mut parser = parser::get_parser(game);
    if let Err(e) = parser.parse(rules_dir) {
        error!("Parser init failed: {}", e);
        return ExitCode::FAILURE;
    }

    // Print Warnings and Notes
    if parser.warning_rules.is_empty() {
        warn!("No rules found to evaluate");
    } else {
        info!("Evaluating mod list...\n");
        debug!("{:?}", &mods);

        parser.evaluate_plugins(&mods);
        for warning in parser.warnings {
            let rule = warning.rule;
            match rule {
                EWarningRule::Note(n) => {
                    info!("[NOTE]\n{}", n.get_comment());
                    info!("Reference: [{}]", n.plugins.join(";"));
                }
                EWarningRule::Conflict(c) => {
                    warn!("[CONFLICT]\n{}", c.get_comment());
                    info!("Reference: [{}]", c.plugins.join(";"));
                }
                EWarningRule::Requires(r) => {
                    error!("[REQUIRES]\n{}", r.get_comment());
                    info!("Reference: [{}]", r.plugins.join(";"));
                }
                EWarningRule::Patch(p) => {
                    warn!("[Patch]\n{}", p.get_comment());
                    info!("Reference: [{}]", p.plugins.join(";"));
                }
            }
            println!();
        }
    }

    // Sort
    if parser.order_rules.is_empty() {
        warn!("No rules found to sort");
        ExitCode::SUCCESS
    } else {
        info!("Sorting mods...");
        let mut sorter = if unstable {
            sorter::new_unstable_sorter()
        } else {
            sorter::new_stable_sorter()
        };

        // check order first
        // match check_order(&mods, &parser.order_rules) {
        //     true => {
        //         // exit
        //         info!("Mods are in correct order, no sorting needed.");
        //         return ExitCode::SUCCESS;
        //     }
        //     false => {}
        // }

        match sorter.topo_sort(game, &mods, &parser.order_rules, &parser.warning_rules) {
            Ok(result) => {
                if dry_run {
                    info!("Dry run...");

                    debug!("Old:\n{:?}", &mods);
                    debug!("New:\n{:?}", result);

                    if mods
                        .iter()
                        .map(|f| f.name.to_lowercase())
                        .collect::<Vec<_>>()
                        .eq(&result)
                    {
                        info!("Mods are in correct order, no sorting needed.");
                    } else {
                        info!("New order:\n{:?}", result);
                    }

                    ExitCode::SUCCESS
                } else {
                    info!("Current:\n{:?}", &mods);

                    if mods
                        .iter()
                        .map(|f| f.name.to_lowercase())
                        .collect::<Vec<_>>()
                        .eq(&result)
                    {
                        info!("Mods are in correct order, no sorting needed.");
                        ExitCode::SUCCESS
                    } else {
                        info!("New:\n{:?}", result);

                        match update_new_load_order(game, &result, config) {
                            Ok(_) => {
                                info!("Update successful");
                                ExitCode::SUCCESS
                            }
                            Err(e) => {
                                error!("Could not updae load order: {}", e);
                                ExitCode::FAILURE
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("error sorting: {e:?}");
                ExitCode::FAILURE
            }
        }
    }
}
