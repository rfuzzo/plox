use clap::{Parser, Subcommand};
use cmop::parser::parse_rules_from_path;
use std::path::PathBuf;
use std::process::ExitCode;

use cmop::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Lists the current mod load order
    List {
        /// Root game folder ("Cyberpunk 2077"). Default is current working directory
        #[arg(default_value_t = String::from("./"))]
        root: String,
    },
    /// Sorts the current mod load order according to specified rules
    Sort {
        /// Root game folder ("Cyberpunk 2077"). Default is current working directory
        #[arg(default_value_t = String::from("./"))]
        root: String,

        /// Folder to read sorting rules from. Default is ./cmop
        #[arg(short, long, default_value_t = String::from("./cmop"))]
        rules_dir: String,

        /// Just print the suggested load order without sorting
        #[arg(short, long)]
        dry_run: bool,

        /// Read the input mods from a file instead of checking the root folder
        #[arg(short, long)]
        mod_list: Option<PathBuf>,
    },
    /// Verifies integrity of the specified rules
    Verify {
        /// Folder to read sorting rules from. Default is ./cmop
        #[arg(short, long, default_value_t = String::from("./cmop"))]
        rules_dir: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::List { root }) => list_mods(&root.into()),
        Some(Commands::Verify { rules_dir }) => {
            //
            let rules_path = PathBuf::from(rules_dir).join("cmop_rules_base.txt");
            verify(&rules_path)
        }
        Some(Commands::Sort {
            root,
            rules_dir,
            mod_list,
            dry_run,
        }) => sort(&root.into(), &rules_dir.into(), mod_list, *dry_run),
        None => ExitCode::FAILURE,
    }
}

/// Sorts the current mod load order according to specified rules
fn sort(
    root: &PathBuf,
    rules_path: &PathBuf,
    mod_list: &Option<PathBuf>,
    dry_run: bool,
) -> ExitCode {
    // gather mods (optionally from a list)
    let mods: Vec<String>;
    if let Some(modlist_path) = mod_list {
        mods = read_file_as_list(modlist_path);
    } else {
        match gather_mods(root) {
            Ok(m) => mods = m,
            Err(e) => {
                println!("No mods found: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    match parse_rules_from_path(rules_path) {
        Ok(rules) => match topo_sort(&mods, &get_order_from_rules(&rules)) {
            Ok(result) => {
                if dry_run {
                    println!("Dry run...");
                    println!("{result:?}");
                } else {
                    println!("Sorting mods...");
                    println!("{:?}", &mods);
                    println!("New:");
                    println!("{result:?}");

                    //todo!()
                }

                ExitCode::SUCCESS
            }
            Err(e) => {
                println!("error sorting: {e:?}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            println!("error parsing file: {e:?}");
            ExitCode::FAILURE
        }
    }
}

/// Verifies integrity of the specified rules
fn verify(rules_path: &PathBuf) -> ExitCode {
    //println!("Verifying rules from {} ...", rules_path.display());

    match parse_rules_from_path(rules_path) {
        Ok(rules) => {
            let order = get_order_from_rules(&rules);
            let mods = get_mods_from_rules(&order);
            match topo_sort(&mods, &order) {
                Ok(_) => {
                    println!("true");
                    ExitCode::SUCCESS
                }
                Err(_) => {
                    println!("false");
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            println!("error parsing file: {e:?}");
            ExitCode::FAILURE
        }
    }
}

/// Lists the current mod load order
fn list_mods(root: &PathBuf) -> ExitCode {
    //println!("Printing active mods...");

    match gather_mods(root) {
        Ok(mods) => {
            for m in mods {
                println!("{}", m);
            }

            ExitCode::SUCCESS
        }
        _ => ExitCode::FAILURE,
    }
}
