use std::env;
use std::fs::{self, File};
use std::io::BufRead;
use std::io::{self};
use std::path::{Path, PathBuf};

use clap::ValueEnum;

pub mod expressions;
pub mod parser;
pub mod rules;
pub mod sorter;

use log::{error, info, warn};
use rules::*;

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum ELogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

pub fn log_level_to_str(level: ELogLevel) -> String {
    match level {
        ELogLevel::Trace => "trace".into(),
        ELogLevel::Debug => "debug".into(),
        ELogLevel::Info => "info".into(),
        ELogLevel::Warn => "warn".into(),
        ELogLevel::Error => "error".into(),
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ESupportedGame {
    Morrowind,
    OpenMorrowind,
    Cyberpunk,
}
pub const PLOX_RULES_BASE: &str = "plox_base.txt";

////////////////////////////////////////////////////////////////////////
/// GAMES
////////////////////////////////////////////////////////////////////////

/// flattens a list of ordered mod pairs into a list of mod names
pub fn debug_get_mods_from_rules(order: &[(String, String)]) -> Vec<String> {
    let mut result: Vec<String> = vec![];
    for (a, b) in order.iter() {
        //TODO wildcards
        if a.contains('?') {
            continue;
        }
        if b.contains('?') {
            continue;
        }

        if a.contains("<VER>") {
            continue;
        }
        if b.contains("<VER>") {
            continue;
        }

        for a in vec![a, b] {
            if a.contains('*') {
                let name1 = a.replace("*", "");
                if !result.contains(&name1) {
                    result.push(name1.to_owned());
                }
            } else if !result.contains(a) {
                result.push(a.to_owned());
            }
        }
    }
    result
}

/// Gets the default rules dir for a game
///
/// # Errors
///
/// This function will return an error if .
pub fn get_default_rules_dir(game: ESupportedGame) -> PathBuf {
    match game {
        ESupportedGame::Morrowind | ESupportedGame::OpenMorrowind => PathBuf::from("mlox"),
        ESupportedGame::Cyberpunk => PathBuf::from("plox"),
    }
}

/// Download latest rules from the internet
pub fn download_latest_rules(game: ESupportedGame, rules_dir: &PathBuf) {
    match game {
        ESupportedGame::Morrowind | ESupportedGame::OpenMorrowind => download_mlox_rules(rules_dir),
        ESupportedGame::Cyberpunk => download_plox_rules(rules_dir),
    }
}

fn download_plox_rules(rules_dir: &PathBuf) {
    match fs::create_dir_all(rules_dir) {
        Ok(_) => {
            // download
            todo!()
        }
        Err(e) => {
            error!(
                "Could not create rules directory at {}: {}",
                rules_dir.display(),
                e
            );
        }
    }
}

fn download_mlox_rules(rules_dir: &PathBuf) {
    match fs::create_dir_all(rules_dir) {
        Ok(_) => {
            // download
            todo!()
        }
        Err(e) => {
            error!(
                "Could not create rules directory at {}: {}",
                rules_dir.display(),
                e
            );
        }
    }
}

/// Gets a list of mod names from the game root folder
///
/// # Errors
///
/// This function will return an error if IO operations fail
pub fn gather_mods<P>(root: &P, game: ESupportedGame) -> Vec<String>
where
    P: AsRef<Path>,
{
    match game {
        ESupportedGame::Morrowind => gather_tes3_mods(root),
        ESupportedGame::Cyberpunk => gather_cp77_mods(root),
        ESupportedGame::OpenMorrowind => gather_openmw_mods(root),
    }
}

/// Get all plugins (esp, omwaddon, omwscripts) in a folder
fn get_plugins_in_folder<P>(path: &P, use_omw_plugins: bool) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    // get all plugins
    let mut results: Vec<PathBuf> = vec![];
    if let Ok(plugins) = std::fs::read_dir(path) {
        plugins.for_each(|p| {
            if let Ok(file) = p {
                let file_path = file.path();
                if file_path.is_file() {
                    if let Some(ext_os) = file_path.extension() {
                        let ext = ext_os.to_ascii_lowercase();
                        if ext == "esm"
                            || ext == "esp"
                            || (use_omw_plugins && ext == "omwaddon")
                            || (use_omw_plugins && ext == "omwscripts")
                        {
                            results.push(file_path);
                        }
                    }
                }
            }
        });
    }
    results
}

fn get_plugins_sorted<P>(path: &P, use_omw_plugins: bool) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    // get plugins
    let mut plugins = get_plugins_in_folder(path, use_omw_plugins);

    // sort
    plugins.sort_by(|a, b| {
        fs::metadata(a.clone())
            .expect("filetime")
            .modified()
            .unwrap()
            .cmp(
                &fs::metadata(b.clone())
                    .expect("filetime")
                    .modified()
                    .unwrap(),
            )
    });
    plugins
}

#[macro_use]
extern crate ini;

pub fn gather_tes3_mods<P>(path: &P) -> Vec<String>
where
    P: AsRef<Path>,
{
    let files = get_plugins_sorted(&path.as_ref().join("Data Files"), false);
    let names = files
        .iter()
        .filter_map(|f| {
            if let Some(file_name) = f.file_name().and_then(|n| n.to_str()) {
                return Some(file_name.to_owned());
            }
            None
        })
        .collect::<Vec<_>>();

    // check against mw ini
    let morrowind_ini_path = PathBuf::from("Morrowind.ini");
    if morrowind_ini_path.exists() {
        // parse ini
        let path = morrowind_ini_path.to_str().expect("Invalid path string");
        let map = ini!(path);
        let mut final_files: Vec<String> = vec![];
        if let Some(section) = map.get("game files") {
            let mods_in_ini = section
                .values()
                .flatten()
                .map(|f| f.to_owned())
                .collect::<Vec<_>>();

            for plugin_name in names {
                if mods_in_ini.contains(&plugin_name) {
                    final_files.push(plugin_name.to_owned());
                }
            }

            return final_files;
        }
        warn!("Morrowind.ini found but no [Game Files] section, using all plugins in Data Files");
    } else {
        warn!("No Morrowind.ini found, using all plugins in Data Files");
    }

    info!("Found {} active plugins", names.len());
    names
}

pub fn gather_openmw_mods<P>(_path: &P) -> Vec<String>
where
    P: AsRef<Path>,
{
    // parse cfg
    if let Ok(cfg) = openmw_cfg::get_config() {
        if let Ok(files) = openmw_cfg::get_plugins(&cfg) {
            let names = files
                .iter()
                .filter_map(|f| {
                    if let Some(file_name) = f.file_name().and_then(|n| n.to_str()) {
                        return Some(file_name.to_owned());
                    }
                    None
                })
                .collect::<Vec<_>>();
            return names;
        }
    }

    vec![]
}

pub fn gather_cp77_mods<P>(root: &P) -> Vec<String>
where
    P: AsRef<Path>,
{
    // gather mods from archive/pc/mod
    let archive_path = root.as_ref().join("archive").join("pc").join("mod");

    if let Ok(plugins) = fs::read_dir(archive_path) {
        let mut entries = plugins
            .map(|res| res.map(|e| e.path()))
            .filter_map(Result::ok)
            .filter_map(|e| {
                if !e.is_dir() {
                    if let Some(os_ext) = e.extension() {
                        if let Some(ext) = os_ext.to_ascii_lowercase().to_str() {
                            if ext.contains("archive") {
                                if let Some(file_name) = e.file_name().and_then(|n| n.to_str()) {
                                    return Some(file_name.to_owned());
                                }
                            }
                        }
                    }
                }
                None
            })
            .collect::<Vec<_>>();

        // TODO support modlist

        // TODO gather REDmods from mods/<NAME>
        entries.sort();
        return entries;
    }

    vec![]
}

/// Update on disk
pub fn update_new_load_order(game: ESupportedGame, result: Vec<String>) {
    match game {
        ESupportedGame::Morrowind => update_tes3(result),
        ESupportedGame::OpenMorrowind => update_openmw(result),
        ESupportedGame::Cyberpunk => update_cp77(result),
    }
}

fn update_cp77(_result: Vec<String>) {
    todo!()
}

fn update_openmw(_result: Vec<String>) {
    todo!()
}

fn update_tes3(_result: Vec<String>) {
    todo!()
}

////////////////////////////////////////////////////////////////////////
/// HELPERS
////////////////////////////////////////////////////////////////////////

/// Extracts a list of ordering-pairs from the order rules
pub fn get_order_rules(rules: &Vec<Rule>) -> Vec<(String, String)> {
    let mut order: Vec<(String, String)> = vec![];
    for r in rules {
        if let Rule::Order(o) = r {
            order.push((o.name_a.to_owned(), o.name_b.to_owned()));
        }
    }

    order
}

pub fn is_current_directory_name(name_to_check: &str) -> bool {
    // Get the current directory
    if let Ok(current_dir) = env::current_dir() {
        // Extract the directory name
        if let Some(dir_name) = current_dir.file_name() {
            // Convert the directory name to a string
            if let Some(name) = dir_name.to_str() {
                return name == name_to_check;
            }
        }
    }

    false
}

/// Returns an Iterator to the Reader of the lines of the file.
///
/// # Errors
///
/// This function will return an error if file io fails
pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

/// read file line by line into vector
pub fn read_file_as_list<P>(modlist_path: P) -> Vec<String>
where
    P: AsRef<Path>,
{
    let mut result: Vec<String> = vec![];
    if let Ok(lines) = read_lines(modlist_path) {
        for line in lines.flatten() {
            result.push(line);
        }
    }
    result
}

pub fn wild_contains(list: &[String], str: &String) -> Option<Vec<String>> {
    if str.contains('*') {
        let mut results = vec![];
        // Replace * with .* to match any sequence of characters
        let mut regex_pattern = str.replace("*", ".*");
        regex_pattern = format!("^{}$", regex_pattern);
        if let Ok(regex) = regex::Regex::new(&regex_pattern) {
            for item in list {
                // Check if the item matches the pattern
                if regex.is_match(item) {
                    //return true;
                    results.push(item.to_owned());
                }
            }
        } else {
            log::error!("Could not construct wildcard pattern for {}", str);
            return None;
        }

        return Some(results);
    }

    if list.contains(str) {
        return Some(vec![str.to_owned()]);
    }

    None
}
