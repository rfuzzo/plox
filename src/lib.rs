use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use clap::ValueEnum;

pub mod expressions;
pub mod parser;
pub mod rules;
pub mod sorter;

use ini::Ini;
use log::{error, info, warn};
use openmw_cfg::config_path;
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

/// Detect game from current working directory
pub fn detect_game() -> Option<ESupportedGame> {
    if PathBuf::from("Morrowind.exe").exists() {
        Some(ESupportedGame::Morrowind)
    } else if PathBuf::from("openmw.exe").exists() {
        Some(ESupportedGame::OpenMorrowind)
    } else if PathBuf::from("x64").join("Cyberpunk2077").exists() {
        Some(ESupportedGame::Cyberpunk)
    } else {
        None
    }
}

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

    // get response body
    let body = &mut response.bytes().unwrap();

    // hash check
    let hash_path = output_path.as_ref().with_extension("hash");
    if hash_path.exists() {
        // check against remote hash
        let local_hash_bytes = fs::read(&hash_path)?;
        let local_hash_str = String::from_utf8_lossy(&local_hash_bytes).to_string();
        if let Ok(local_hash) = local_hash_str.parse::<u64>() {
            let remote_hash = seahash::hash(body);
            if local_hash == remote_hash {
                // return
                info!(
                    "File already is latest version: {}",
                    output_path.as_ref().display()
                );
                return Ok(());
            }
        }
    }

    // Create a file and write

    let mut file = File::create(output_path)?;
    io::copy(&mut body.as_ref(), &mut file)?;
    info!(
        "File downloaded successfully: {}",
        output_path.as_ref().display()
    );

    // create hash
    let remote_hash = seahash::hash(body);
    fs::write(hash_path, remote_hash.to_string())?;

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
                    Ok(()) => {}
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
            // TODO CP77 download plox rules
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
        if let Ok(ini) = Ini::load_from_file(morrowind_ini_path) {
            let mut final_files: Vec<String> = vec![];
            if let Some(section) = ini.section(Some("Game Files")) {
                let mods_in_ini: Vec<_> = section.iter().map(|f| f.1).collect();
                for plugin_name in names {
                    if mods_in_ini.contains(&plugin_name.as_str()) {
                        final_files.push(plugin_name.to_owned());
                    }
                }

                return final_files;
            }
            warn!(
                "Morrowind.ini found but no [Game Files] section, using all plugins in Data Files"
            );
        } else {
            error!("Morrowind.ini could not be read");
        }
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
    } else {
        error!("No openmw.cfg found");
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
pub fn update_new_load_order(game: ESupportedGame, result: &[String]) -> std::io::Result<()> {
    match game {
        ESupportedGame::Morrowind => update_tes3(result),
        ESupportedGame::OpenMorrowind => update_openmw(result),
        ESupportedGame::Cyberpunk => update_cp77(result),
    }
}

fn update_cp77(_result: &[String]) -> std::io::Result<()> {
    // TODO CP77 update mods
    panic!("Not implemented")
}

fn update_openmw(result: &[String]) -> std::io::Result<()> {
    // in openMW we just update the cfg with the new order
    if let Ok(_cfg) = openmw_cfg::get_config() {
        // parse ini
        let mut buf = Vec::new();
        for line in read_lines(config_path())?.map_while(Result::ok) {
            // skip plugin lines

            if line.starts_with("content=") {
                continue;
            }
            writeln!(buf, "{}", line)?;
        }

        // add filenames
        for r in result {
            writeln!(buf, "content={}", r)?;
        }

        // save
        let mut file = File::create(config_path())?;
        file.write_all(&buf)?;
    } else {
        error!("No openmw.cfg found");
    }

    Ok(())
}

fn update_tes3(result: &[String]) -> std::io::Result<()> {
    // in tes3 we first update the ini with the new order (this is technically not important but we might as well)
    // check against mw ini
    let morrowind_ini_path = PathBuf::from("Morrowind.ini");
    if morrowind_ini_path.exists() {
        // parse ini
        let mut buf = Vec::new();
        for line in read_lines(&morrowind_ini_path)?.map_while(Result::ok) {
            // skip plugin lines
            if line.starts_with("[Game Files]") {
                continue;
            }
            if line.starts_with("GameFile") {
                continue;
            }
            writeln!(buf, "{}", line)?;
        }

        // add filenames
        writeln!(buf, "[Game Files]")?;
        for (i, r) in result.iter().enumerate() {
            writeln!(buf, "GameFile{}={}", i, r)?;
        }

        // save
        let mut file = File::create(morrowind_ini_path)?;
        file.write_all(&buf)?;
    } else {
        warn!("No Morrowind.ini found, using all plugins in Data Files");
    }

    // then actually reset the filetimes on all plugins hooray

    Ok(())
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

/// Checks if the list contains the str
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

    // #[test]
    // fn test_update_openmw() {
    //     let result = ["a".to_owned(), "b".to_owned(), "c".to_owned()];
    //     update_openmw(&result).expect("write failed");
    // }

    // #[test]
    // fn test_update_tes3() {
    //     let result = ["a".to_owned(), "b".to_owned(), "c".to_owned()];
    //     update_tes3(&result).expect("write failed");
    // }

    // #[test]
    // fn test_update_tes3() {
    //     let morrowind_ini_path = PathBuf::from("Morrowind.ini");
    //     if morrowind_ini_path.exists() {
    //         // parse ini
    //         if let Ok(ini) = Ini::load_from_file(morrowind_ini_path) {
    //             if let Some(section) = ini.section(Some("Game Files")) {
    //                 for m in section.iter().map(|f| f.1) {
    //                     eprintln!("{}", m);
    //                 }
    //             }
    //             warn!(
    //             "Morrowind.ini found but no [Game Files] section, using all plugins in Data Files"
    //         );
    //         } else {
    //             error!("Morrowind.ini could not be read");
    //         }
    //     } else {
    //         warn!("No Morrowind.ini found, using all plugins in Data Files");
    //     }
    // }
}
