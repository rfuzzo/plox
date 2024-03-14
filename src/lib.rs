use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufRead, Read, Seek, Write};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::{env, vec};

use clap::ValueEnum;

pub mod expressions;
pub mod parser;
pub mod rules;
pub mod sorter;

use byteorder::{LittleEndian, ReadBytesExt};
use filetime::set_file_mtime;
use ini::Ini;
use log::{error, info, warn};
use openmw_cfg::config_path;
use rules::*;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Serialize, Deserialize)]
pub enum ESupportedGame {
    Morrowind,
    OpenMW,
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
    } else if PathBuf::from("openmw.cfg").exists() {
        Some(ESupportedGame::OpenMW)
    } else if PathBuf::from("bin")
        .join("x64")
        .join("Cyberpunk2077")
        .exists()
    {
        Some(ESupportedGame::Cyberpunk)
    } else {
        None
    }
}

/// flattens a list of ordered mod pairs into a list of mod names
pub fn debug_get_mods_from_order_rules(order_rules: &[EOrderRule]) -> Vec<PluginData> {
    debug_get_mods_from_ordering(&get_ordering_from_order_rules(order_rules))
}

/// flattens a list of ordered mod pairs into a list of mod names
pub fn debug_get_mods_from_ordering(order: &[(String, String)]) -> Vec<PluginData> {
    let mut result: Vec<PluginData> = vec![];
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

            let data = PluginData::new(name, 0);
            if !result.contains(&data) {
                result.push(data);
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
        ESupportedGame::Morrowind | ESupportedGame::OpenMW => PathBuf::from("mlox"),
        ESupportedGame::Cyberpunk => PathBuf::from("plox"),
    }
}

/// Download latest rules from the internet
pub fn download_latest_rules(game: ESupportedGame, rules_dir: &PathBuf) {
    match game {
        ESupportedGame::Morrowind | ESupportedGame::OpenMW => download_mlox_rules(rules_dir),
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

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct PluginData {
    pub name: String,
    pub size: u64,

    pub description: Option<String>,
    pub version: Option<semver::Version>,
}

impl PluginData {
    pub fn new(name: String, size: u64) -> Self {
        Self {
            name,
            size,
            description: None,
            version: None,
        }
    }
}

/// Gets a list of mod names from the game root folder
///
/// # Errors
///
/// This function will return an error if IO operations fail
pub fn gather_mods<P>(root: &P, game: ESupportedGame, config: Option<P>) -> Vec<PluginData>
where
    P: AsRef<Path>,
{
    match game {
        ESupportedGame::Morrowind => gather_tes3_mods(root),
        ESupportedGame::Cyberpunk => gather_cp77_mods(root),
        ESupportedGame::OpenMW => gather_openmw_mods(&config),
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

pub fn gather_tes3_mods<P>(path: &P) -> Vec<PluginData>
where
    P: AsRef<Path>,
{
    let files = get_plugins_sorted(&path.as_ref().join("Data Files"), false);
    let names = files
        .iter()
        .filter_map(|f| {
            if let Some(file_name) = f.file_name().and_then(|n| n.to_str()) {
                let mut data = PluginData {
                    name: file_name.to_owned(),
                    size: f.metadata().unwrap().len(),
                    description: None,
                    version: None,
                };

                // skip if extension is omwscripts
                if !file_name.to_ascii_lowercase().ends_with("omwscripts") {
                    match parse_header(f) {
                        Ok(header) => {
                            data.description = Some(header.description);

                            // parse semver
                            let version = header.version.to_string();
                            match lenient_semver::parse(&version) {
                                Ok(v) => {
                                    data.version = Some(v);
                                }
                                Err(e) => {
                                    log::debug!("Error parsing version: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            log::debug!("Error parsing header: {}, {}", e, f.display());
                        }
                    }
                }

                return Some(data);
            }
            None
        })
        .collect::<Vec<_>>();

    // check against mw ini
    let morrowind_ini_path = PathBuf::from("Morrowind.ini");
    if morrowind_ini_path.exists() {
        // parse ini
        if let Ok(ini) = Ini::load_from_file(morrowind_ini_path) {
            let mut final_files: Vec<PluginData> = vec![];
            if let Some(section) = ini.section(Some("Game Files")) {
                let mods_in_ini: Vec<_> = section.iter().map(|f| f.1).collect();
                for data in names {
                    if mods_in_ini.contains(&data.name.as_str()) {
                        final_files.push(data.clone());
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

pub fn gather_openmw_mods<P>(config: &Option<P>) -> Vec<PluginData>
where
    P: AsRef<Path>,
{
    // parse cfg
    let mut path = config_path();
    if let Some(config_path) = config {
        if config_path.as_ref().exists() {
            path = config_path.as_ref().to_path_buf();
        } else {
            error!("openmw.cfg not found at {}", config_path.as_ref().display());
        }
    }

    if let Ok(cfg) = openmw_cfg::Ini::load_from_file_noescape(path) {
        if let Ok(files) = openmw_cfg::get_plugins(&cfg) {
            let names = files
                .iter()
                .filter_map(|f| {
                    if let Some(file_name) = f.file_name().and_then(|n| n.to_str()) {
                        let mut data = PluginData {
                            name: file_name.to_owned(),
                            size: f.metadata().unwrap().len(),
                            description: None,
                            version: None,
                        };
                        match parse_header(f) {
                            Ok(header) => {
                                data.description = Some(header.description);

                                // parse semver
                                let version = header.version.to_string();
                                match lenient_semver::parse(&version) {
                                    Ok(v) => {
                                        data.version = Some(v);
                                    }
                                    Err(e) => {
                                        log::debug!("Error parsing version: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                log::debug!("Error parsing header: {}, {}", e, f.display());
                            }
                        }
                        return Some(data);
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

pub fn gather_cp77_mods<P>(root: &P) -> Vec<PluginData>
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
                                    let data = PluginData {
                                        name: file_name.to_owned(),
                                        size: e.metadata().unwrap().len(),
                                        description: None,
                                        version: None,
                                    };
                                    return Some(data);
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
        entries.sort_by_key(|e| e.name.clone());
        return entries;
    }

    vec![]
}

/// Update on disk
pub fn update_new_load_order<P: AsRef<Path>>(
    game: ESupportedGame,
    result: &[String],
    config: Option<P>,
) -> std::io::Result<()> {
    match game {
        ESupportedGame::Morrowind => update_tes3(result),
        ESupportedGame::OpenMW => update_openmw(result, config),
        ESupportedGame::Cyberpunk => update_cp77(result),
    }
}

fn update_cp77(_result: &[String]) -> std::io::Result<()> {
    // TODO CP77 update mods
    panic!("Not implemented")
}

fn update_openmw<P: AsRef<Path>>(result: &[String], config: Option<P>) -> std::io::Result<()> {
    // in openMW we just update the cfg with the new order
    let mut path = config_path();
    if let Some(config_path) = config {
        if config_path.as_ref().exists() {
            path = config_path.as_ref().to_path_buf();
        } else {
            error!("openmw.cfg not found at {}", config_path.as_ref().display());
        }
    }

    if let Ok(_cfg) = openmw_cfg::Ini::load_from_file_noescape(&path) {
        // parse ini
        let mut buf = Vec::new();
        for line in read_lines(&path)?.map_while(Result::ok) {
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
        let mut file = File::create(path)?;
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

    // redate files
    let files = result
        .iter()
        .map(|f| PathBuf::from("Data Files").join(f))
        .collect::<Vec<_>>();
    redate_mods(&files)?;

    Ok(())
}

/// Checks if the list of mods is in the correct order
pub fn check_order(result: &[String], order_rules: &[EOrderRule]) -> bool {
    let order = get_ordering_from_order_rules(order_rules);
    let pairs = order;
    for (a, b) in pairs {
        if let Some(results_for_a) = wild_contains(result, &a) {
            if let Some(results_for_b) = wild_contains(result, &b) {
                for i in &results_for_a {
                    for j in &results_for_b {
                        let pos_a = result.iter().position(|x| x == i).unwrap();
                        let pos_b = result.iter().position(|x| x == j).unwrap();
                        if pos_a > pos_b {
                            return false;
                        }
                    }
                }
            }
        }
    }

    true
}

////////////////////////////////////////////////////////////////////////
/// TES3
////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone, Default)]
pub struct Tes3Header {
    pub version: f32,
    pub description: String,
    pub masters: Option<Vec<(String, u64)>>,
}

pub fn parse_header(f: &Path) -> std::io::Result<Tes3Header> {
    let magic: u32 = 861095252;
    // read file to binary reader
    let mut reader = std::io::BufReader::new(std::fs::File::open(f)?);
    // read first 4 bytes and check magic
    let file_magic = reader.read_u32::<LittleEndian>()?;
    if file_magic != magic {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Not a valid TES3 plugin",
        ));
    }

    // next 4 bytes is the size of the header
    let header_size = reader.read_u32::<LittleEndian>()?;
    // skip 8 bytes
    reader.seek(std::io::SeekFrom::Current(8))?;
    // read the header
    let mut header_buffer = vec![0; header_size as usize];
    reader.read_exact(&mut header_buffer)?;

    let mut reader = std::io::Cursor::new(header_buffer);
    let header = parse_hedr(&mut reader, header_size as u64)?;
    Ok(header)
}

fn parse_hedr<R: Read + Seek>(reader: &mut R, stream_size: u64) -> std::io::Result<Tes3Header> {
    let magic: u32 = 1380205896;
    // check magic
    let file_magic = reader.read_u32::<LittleEndian>()?;

    if file_magic != magic {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Not a valid TES3 plugin",
        ));
    }

    let mut header = Tes3Header::default();

    // next 4 bytes is the size of the header
    let _header_size = reader.read_u32::<LittleEndian>()?;

    // next 4 bytes is the version
    header.version = reader.read_f32::<LittleEndian>()?;

    // next 4 bytes is unused
    let _ = reader.read_u32::<LittleEndian>()?;

    // read 32 bytes as string
    let mut string_buffer = [0; 32];
    reader.read_exact(&mut string_buffer)?;
    let _author = String::from_utf8_lossy(&string_buffer).to_string();

    // read 256 bytes as string
    let mut string_buffer = [0; 256];
    reader.read_exact(&mut string_buffer)?;
    header.description = String::from_utf8_lossy(&string_buffer)
        .trim_end_matches('\0')
        .to_string();

    // read 4 bytes as u32
    let _num_records = reader.read_u32::<LittleEndian>()?;

    let master_magic: u32 = 1414742349;
    let data_magic: u32 = 1096040772;

    // read masters
    if reader.stream_position()? >= stream_size {
        return Ok(header);
    }

    let mut masters = vec![];
    loop {
        let magic = reader.read_u32::<LittleEndian>()?;
        if magic == master_magic {
            // next 4 bytes is the size of the master string name
            let master_size = reader.read_u32::<LittleEndian>()?;
            // read master name
            let mut master_buffer = vec![0; master_size as usize];
            reader.read_exact(&mut master_buffer)?;
            let master_name = String::from_utf8_lossy(&master_buffer).to_string();

            // read data magic
            let magic_data = reader.read_u32::<LittleEndian>()?;
            if magic_data != data_magic {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Not a valid TES3 plugin",
                ));
            }
            // next 4 bytes is the size of the master data
            let master_data_size = reader.read_u32::<LittleEndian>()?;
            // verify master data size is 8
            if master_data_size != 8 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Not a valid TES3 plugin",
                ));
            }

            // next 8 bytes is size
            let size = reader.read_u64::<LittleEndian>()?;

            masters.push((master_name.trim_end_matches('\0').to_string(), size));

            // break out if end of stream
            if reader.stream_position()? >= stream_size {
                break;
            }
        } else {
            break;
        }
    }
    header.masters = Some(masters);

    Ok(header)
}

fn redate_mods(files: &[PathBuf]) -> Result<(), io::Error> {
    let fixed_file_times: HashMap<String, usize> = HashMap::from([
        ("morrowind.esm".into(), 1024695106),
        ("tribunal.esm".into(), 1035940926),
        ("bloodmoon.esm".into(), 1051807050),
    ]);
    let start_time = 1024695106;
    let mut current_time = start_time;
    for mod_path in files {
        // Change the modification times of plugin files to be in order of file list, oldest to newest
        // check if is a fixed file time file
        let filename = mod_path.file_name().unwrap().to_str().unwrap();
        if let Some(time) = fixed_file_times.get(&filename.to_lowercase()) {
            let time = *time as i64;
            current_time = time;
            set_file_mtime(mod_path, filetime::FileTime::from_unix_time(time, 0))?;
        } else {
            // set the time to start time + 60
            current_time += 60;
            set_file_mtime(
                mod_path,
                filetime::FileTime::from_unix_time(current_time, 0),
            )?;
        }
    }

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
pub fn read_file_as_list<P>(modlist_path: P) -> Vec<PluginData>
where
    P: AsRef<Path>,
{
    let mut result: Vec<PluginData> = vec![];
    if let Ok(lines) = read_lines(modlist_path) {
        for line in lines.map_while(Result::ok) {
            let data = PluginData {
                name: line,
                size: 0, // TODO fix dummy size
                description: None,
                version: None,
            };
            result.push(data);
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

/// Checks if the list contains the str
pub fn wild_contains_data(list: &[PluginData], str: &String) -> Option<Vec<PluginData>> {
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
                if regex.is_match(&item.name) {
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

    if let Some(r) = list.iter().find(|f| f.name.eq(str)) {
        return Some(vec![r.to_owned()]);
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
    //use std::fs::create_dir_all;

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

    // #[test]
    // fn test_redate_mods() {
    //     let result = [
    //         "morrowind.esm".to_owned(),
    //         "tribunal.esm".to_owned(),
    //         "bloodmoon.esm".to_owned(),
    //         "a.esp".to_owned(),
    //         "b.esp".to_owned(),
    //         "c.esp".to_owned(),
    //     ];

    //     // create the files in /tmp
    //     create_dir_all("tmp").expect("copuld not create dir");
    //     let mut files = vec![];
    //     for r in &result {
    //         let mod_path = PathBuf::from("tmp").join(r);
    //         let _ = File::create(&mod_path);
    //         files.push(mod_path.clone());
    //     }

    //     redate_mods(&files).expect("redate failed");

    //     // check if the filetime is correct
    //     for path in &files {
    //         let metadata = fs::metadata(path).expect("metadata failed");
    //         let modified = metadata.modified().expect("modified failed");
    //         let unix_time = filetime::FileTime::from_system_time(modified);
    //         eprintln!("{} - {:?}", path.display(), unix_time);
    //     }

    //     // delete the files again
    //     for path in &files {
    //         fs::remove_file(path).expect("remove failed");
    //     }
    // }
}
