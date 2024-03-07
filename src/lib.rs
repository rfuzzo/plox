use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io;
use std::io::BufRead;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::ValueEnum;

pub mod expressions;
pub mod parser;
pub mod rules;
pub mod sorter;

use log::{error, info, warn};
use reqwest::header::LAST_MODIFIED;
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

/// Sorts the current mod load order according to specified rules
pub fn sort(
    game: ESupportedGame,
    root: &Option<PathBuf>,
    rules_path: &Option<String>,
    mod_list: &Option<PathBuf>,
    dry_run: bool,
    unstable: bool,
    no_download: bool,
) -> ExitCode {
    info!("Sorting mods...");

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

    // gather mods (optionally from a list)
    let mods: Vec<String>;
    if let Some(modlist_path) = mod_list {
        mods = read_file_as_list(modlist_path);
    } else {
        mods = gather_mods(&root, game);
        if mods.is_empty() {
            info!("No mods found");
            return ExitCode::FAILURE;
        }
    }

    if !no_download {
        download_latest_rules(game, &rules_dir);
    }

    let mut parser = parser::get_parser(game);
    if let Err(e) = parser.init(rules_dir) {
        error!("Parser init failed: {}", e);
        return ExitCode::FAILURE;
    }

    if parser.rules.is_empty() {
        warn!("No rules found to evaluate");
        return ExitCode::FAILURE;
    }

    // Print Warnings and Notes
    info!("Evaluating mod list...");
    for rule in &parser.rules {
        if rule.eval(&mods) {
            match rule {
                EWarningRule::Note(n) => {
                    info!("[NOTE]\n{}\n", n.get_comment());
                }
                EWarningRule::Conflict(c) => {
                    warn!("[CONFLICT]\n{}\n", c.get_comment());
                }
                EWarningRule::Requires(r) => {
                    warn!("[REQUIRES]\n{}\n", r.get_comment());
                }
                EWarningRule::Patch(p) => {
                    warn!("[Patch]\n{}\n", p.get_comment());
                }
            }
        }
    }

    // Sort

    info!("Sorting mods...");
    let mut sorter = if unstable {
        sorter::new_unstable_sorter()
    } else {
        sorter::new_stable_sorter()
    };
    match sorter.topo_sort(&mods, &parser.order_rules) {
        Ok(result) => {
            if dry_run {
                info!("Dry run...");
                info!("New:\n{:?}", result);
            } else {
                info!("Current:\n{:?}", &mods);

                if mods.eq(&result) {
                    info!("Mods are in correct order, no sorting needed.");
                } else {
                    info!("New:\n{:?}", result);

                    update_new_load_order(game, result);
                }
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            error!("error sorting: {e:?}");
            ExitCode::FAILURE
        }
    }
}

/// Verifies integrity of the specified rules
pub fn verify(game: ESupportedGame, rules_path: &Option<String>) -> ExitCode {
    let rules_dir = if let Some(path) = rules_path {
        PathBuf::from(path)
    } else {
        get_default_rules_dir(game)
    };

    info!("Verifying rules from {} ...", rules_dir.display());

    let mut parser = parser::get_parser(game);
    if let Err(e) = parser.init(rules_dir) {
        error!("Parser init failed: {}", e);
        return ExitCode::FAILURE;
    }

    if parser.rules.is_empty() {
        warn!("No rules found to evaluate");
        return ExitCode::FAILURE;
    }

    let mods = debug_get_mods_from_order_rules(&parser.order_rules);
    match sorter::new_unstable_sorter().topo_sort(&mods, &parser.order_rules) {
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

/// Lists the current mod load order
pub fn list_mods(root: &Option<PathBuf>, game: ESupportedGame) -> ExitCode {
    info!("Printing active mods...");

    let root = match root {
        Some(path) => path.clone(),
        None => env::current_dir().expect("No current working dir"),
    };

    for m in gather_mods(&root, game) {
        println!("{}", m);
        //info!("{}", m);
    }

    ExitCode::SUCCESS
}

////////////////////////////////////////////////////////////////////////
/// GAMES
////////////////////////////////////////////////////////////////////////

/// flattens a list of ordered mod pairs into a list of mod names
pub fn debug_get_mods_from_order_rules(order_rules: &[EOrderRule]) -> Vec<String> {
    debug_get_mods_from_ordering(&get_ordering_from_order_rules(order_rules))
}

/// flattens a list of ordered mod pairs into a list of mod names
pub fn debug_get_mods_from_ordering(order: &[(String, String)]) -> Vec<String> {
    let mut result: Vec<String> = vec![];
    for (a, b) in order.iter() {
        for a in [a, b] {
            let name = if a.contains('*') || a.contains('?') || a.contains("<ver>") {
                // Wildcards
                a.replace('?', "x")
                    .replace(['*'], "")
                    .replace("<ver>", "1.0")
            } else {
                a.to_owned()
            };

            if !result.contains(&name) {
                result.push(name);
            }
        }
    }

    result.dedup();

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

fn download_file<P>(url: &str, output_path: &P) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    // Send an HTTP GET request to the URL
    let response = reqwest::blocking::get(url)?;

    // Create a file at the specified output path
    let mut file = File::create(output_path)?;

    // Write the response body to the file
    io::copy(&mut response.bytes().unwrap().as_ref(), &mut file)?;

    Ok(())
}

pub fn download_file_if_different_version(
    url: &str,
    output_path: &str,
    local_version: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    // Create the output directory if it doesn't exist
    if let Some(parent_dir) = Path::new(output_path).parent() {
        fs::create_dir_all(parent_dir)?;
    }

    // Send a HEAD request to check if the file has been modified
    let client = reqwest::blocking::Client::new();
    let response = client.head(url).send()?;

    // Get the Last-Modified header from the response
    if let Some(last_modified) = response.headers().get(LAST_MODIFIED) {
        if let Ok(last_modified_str) = last_modified.to_str() {
            // If the local version is different from the remote version, download the file
            if local_version != Some(last_modified_str) {
                // Send a GET request to download the file
                let response = reqwest::blocking::get(url)?;

                // Create a file at the specified output path
                let mut file = File::create(output_path)?;

                // Write the response body to the file
                io::copy(&mut response.bytes().unwrap().as_ref(), &mut file)?;

                println!("File downloaded successfully.");
                return Ok(());
            }
        }
    }

    println!("Local file is up to date.");
    Ok(())
}

fn download_mlox_rules(rules_dir: &PathBuf) {
    match fs::create_dir_all(rules_dir) {
        Ok(_) => {
            // download
            let repo = "https://github.com/DanaePlays/mlox-rules/raw/main/";
            let files = ["mlox_base.txt", "mlox_user.txt"];
            for file in files {
                let output_path = rules_dir.join(file); // Specify the output path here
                let url = repo.to_owned() + file;
                match download_file(&url, &output_path) {
                    Ok(()) => info!("File downloaded successfully: {}", file),
                    Err(err) => error!("Error downloading file: {}", err),
                }
            }
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
    if let Ok(plugins) = fs::read_dir(path) {
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

        // TODO CP77 support modlist

        // TODO CP77 gather REDmods from mods/<NAME>
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

fn generate_pair_permutations(input: &[String]) -> Vec<(String, String)> {
    let mut permutations = Vec::new();
    for i in 0..input.len() - 1 {
        for j in i + 1..input.len() {
            permutations.push((input[i].to_owned(), input[j].to_owned()));
        }
    }
    permutations
}

fn get_permutations(o: &Order, orders: &mut Vec<(String, String)>) -> ControlFlow<()> {
    // process order rules
    if let std::cmp::Ordering::Less = o.names.len().cmp(&2) {
        // Rule with only one element is an error
        return ControlFlow::Break(());
    }
    orders.extend(generate_pair_permutations(&o.names));
    ControlFlow::Continue(())
}

/// Extracts a list of ordering-pairs from the order rules
pub fn get_ordering(rules: &Vec<ERule>) -> Vec<(String, String)> {
    let mut orders: Vec<(String, String)> = vec![];

    for r in rules {
        if let ERule::EOrderRule(EOrderRule::Order(o)) = r {
            if let ControlFlow::Break(_) = get_permutations(o, &mut orders) {
                continue;
            }
        }
    }

    orders
}

/// Extracts a list of ordering-pairs from the order rules
pub fn get_ordering_from_order_rules(rules: &[EOrderRule]) -> Vec<(String, String)> {
    let mut orders: Vec<(String, String)> = vec![];

    for r in rules {
        if let EOrderRule::Order(o) = r {
            if let ControlFlow::Break(_) = get_permutations(o, &mut orders) {
                continue;
            }
        }
    }

    orders
}

/// Extracts a list of ordering-pairs from the order rules
pub fn get_ordering_from_orders(rules: &Vec<Order>) -> Vec<(String, String)> {
    let mut orders: Vec<(String, String)> = vec![];

    for o in rules {
        // process order rules
        if let ControlFlow::Break(_) = get_permutations(o, &mut orders) {
            continue;
        }
    }

    orders
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
        for line in lines.map_while(Result::ok) {
            result.push(line);
        }
    }
    result
}

pub fn wild_contains(list: &[String], str: &String) -> Option<Vec<String>> {
    if str.contains('*') || str.contains('?') || str.contains("<ver>") {
        let mut results = vec![];
        // Replace * with .* to match any sequence of characters
        let mut regex_pattern = str.replace('*', r".*");
        // Replace ? with . to match any single character
        regex_pattern = regex_pattern.replace('?', r".");
        // Replace <ver> with (\d+(?:[_.-]?\d+)*[a-z]?) to match a version number :hidethepain:
        // the following are valid version numbers: 1.2.3a, 1.0, 1, 1a, 1_3a, 77g
        regex_pattern = regex_pattern.replace("<ver>", r"(\d+(?:[_.-]?\d+)*[a-z]?)");

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

        if results.is_empty() {
            return None;
        }

        return Some(results);
    }

    if list.contains(str) {
        return Some(vec![str.to_owned()]);
    }

    None
}

pub fn note(f: ERule) -> Option<Note> {
    match f {
        ERule::EWarningRule(EWarningRule::Note(n)) => Some(n),
        _ => None,
    }
}

pub fn conflict(f: ERule) -> Option<Conflict> {
    match f {
        ERule::EWarningRule(EWarningRule::Conflict(n)) => Some(n),
        _ => None,
    }
}
pub fn requires(f: ERule) -> Option<Requires> {
    match f {
        ERule::EWarningRule(EWarningRule::Requires(n)) => Some(n),
        _ => None,
    }
}
pub fn patch(f: ERule) -> Option<Patch> {
    match f {
        ERule::EWarningRule(EWarningRule::Patch(n)) => Some(n),
        _ => None,
    }
}

// order
pub fn order(f: ERule) -> Option<Order> {
    match f {
        ERule::EOrderRule(EOrderRule::Order(o)) => Some(o),
        _ => None,
    }
}
pub fn order2(f: EOrderRule) -> Option<Order> {
    match f {
        EOrderRule::Order(o) => Some(o),
        _ => None,
    }
}
pub fn nearstart(f: ERule) -> Option<NearStart> {
    match f {
        ERule::EOrderRule(EOrderRule::NearStart(o)) => Some(o),
        _ => None,
    }
}
pub fn nearstart2(f: &EOrderRule) -> Option<NearStart> {
    match f {
        EOrderRule::NearStart(o) => Some(o.clone()),
        _ => None,
    }
}
pub fn nearend(f: ERule) -> Option<NearEnd> {
    match f {
        ERule::EOrderRule(EOrderRule::NearEnd(o)) => Some(o),
        _ => None,
    }
}
pub fn nearend2(f: &EOrderRule) -> Option<NearEnd> {
    match f {
        EOrderRule::NearEnd(o) => Some(o.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_generate_pair_permutations() {
        {
            let input = ["a".to_owned(), "b".to_owned(), "c".to_owned()];
            let got = generate_pair_permutations(&input);
            let expected = [
                ("a".to_owned(), "b".to_owned()),
                ("a".to_owned(), "c".to_owned()),
                ("b".to_owned(), "c".to_owned()),
            ];
            assert_eq!(got, expected);
        }

        {
            let input = ["a".to_owned(), "b".to_owned()];
            let got = generate_pair_permutations(&input);
            let expected = [("a".to_owned(), "b".to_owned())];
            assert_eq!(got, expected);
        }
    }

    #[test]
    fn test_wildcard_matches_star() {
        let pattern = "Hold it - replacer*.esp".to_lowercase().to_owned();

        {
            let input = "Hold it - replacer.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_some());
        }

        {
            let input = "Hold it - replacerA.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_some());
        }

        {
            let input = "Hold it - replacerAA.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_some());
        }

        // Fails

        {
            let input = "Hold it - replace.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_none());
        }
    }

    #[test]
    fn test_wildcard_matches_questionmark() {
        let pattern = "Rem_LoC?.esp".to_lowercase().to_owned();

        {
            let input = "Rem_LoCA.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_some());
        }

        {
            let input = "Rem_LoC1.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_some());
        }

        // Fails

        {
            let input = "Rem_LoCAA.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_none());
        }

        {
            let input = "Rem_LoC.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_none());
        }
    }

    #[test]
    fn test_wildcard_matches_ver() {
        // the following are valid version numbers: 1.0, 1.2.3a, 1, 1a, 1_3a, 77g
        let pattern = "ADN-GDRv<VER>.esp".to_lowercase().to_owned();

        {
            let input = "ADN-GDRv1.0.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_some());
        }

        {
            let input = "ADN-GDRv1.2.3a.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_some());
        }

        {
            let input = "ADN-GDRv1.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_some());
        }

        {
            let input = "ADN-GDRv1a.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_some());
        }

        {
            let input = "ADN-GDRv1_3a.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_some());
        }

        {
            let input = "ADN-GDRv77g.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_some());
        }

        // Fails

        {
            let input = "ADN-GDRv1.0_comment.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_none());
        }

        {
            let input = "ADN-GDRv.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_none());
        }

        {
            let input = "ADN-GDRvMyE3.esp".to_lowercase().to_owned();
            assert!(wild_contains(&[input], &pattern).is_none());
        }
    }
}
