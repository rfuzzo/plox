use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufRead;
use std::io::{self};
use std::path::Path;
use toposort_scc::IndexGraph;

pub mod expressions;
pub mod rules;

use rules::*;

////////////////////////////////////////////////////////////////////////
/// LOGIC
////////////////////////////////////////////////////////////////////////

pub fn stable_topo_sort_inner(
    n: usize,
    edges: &[(usize, usize)],
    index_dict: &HashMap<&str, usize>,
    result: &mut Vec<String>,
) -> bool {
    for i in 0..n {
        for j in 0..i {
            let x = index_dict[result[i].as_str()];
            let y = index_dict[result[j].as_str()];
            if edges.contains(&(x, y)) {
                let t = result[i].to_owned();
                result.remove(i);
                result.insert(j, t);
                return true;
            }
        }
    }
    false
}

pub fn topo_sort(mods: &Vec<String>, rules: &Rules) -> Result<Vec<String>, &'static str> {
    let mut g = IndexGraph::with_vertices(mods.len());
    let mut index_dict: HashMap<&str, usize> = HashMap::new();
    for (i, m) in mods.iter().enumerate() {
        index_dict.insert(m, i);
    }
    // add edges
    let mut edges: Vec<(usize, usize)> = vec![];
    for (a, b) in &rules.order {
        if mods.contains(a) && mods.contains(b) {
            let idx_a = index_dict[a.as_str()];
            let idx_b = index_dict[b.as_str()];
            g.add_edge(idx_a, idx_b);
            edges.push((idx_a, idx_b));
        }
    }
    // cycle check
    let sort = g.toposort();
    if sort.is_none() {
        return Err("Graph contains a cycle");
    }

    // sort
    let mut result: Vec<String> = mods.iter().map(|e| (*e).to_owned()).collect();
    println!("{result:?}");
    loop {
        if !stable_topo_sort_inner(mods.len(), &edges, &index_dict, &mut result) {
            break;
        }
    }

    // Return the sorted vector
    Ok(result)
}

#[derive(PartialEq)]
enum ERule {
    Order,
    Note,
    Conflict,
    Requires,
}

/// custom rules parser
///
/// # Errors
///
/// This function will return an error if .
pub fn parse_rules<P>(rules_dir: P) -> io::Result<Rules>
where
    P: AsRef<Path>,
{
    let mut rules: Rules = Rules::default();

    let mut order: Vec<(String, String)> = vec![];
    let mut orders: Vec<Vec<String>> = vec![];
    let mut warning_rules: Vec<RuleKind> = vec![];

    // todo scan directory for user files
    let rules_path = rules_dir.as_ref().join("cmop_rules_base.txt");
    let lines = read_lines(rules_path)?;
    let mut parsing = false;
    let mut current_rule_type: Option<ERule> = None;

    let mut current_order: Vec<String> = vec![];
    let mut current_warning_rule: Option<RuleKind> = None;

    // parse each line
    for line in lines.flatten() {
        // comments
        if line.starts_with(';') {
            continue;
        }

        // new empty lines end a rule block
        if parsing && line.is_empty() {
            parsing = false;
            if let Some(current_rule) = current_rule_type.take() {
                // TODO this is stupid
                if current_rule == ERule::Order {
                    orders.push(current_order.to_owned());
                } else if let Some(current_warning_rule) = current_warning_rule.take() {
                    match current_rule {
                        ERule::Order => {}
                        ERule::Note => warning_rules.push(current_warning_rule),
                        ERule::Conflict => warning_rules.push(current_warning_rule),
                        ERule::Requires => warning_rules.push(current_warning_rule),
                    }
                } else {
                    // todo error
                }
                current_order.clear();
            }

            continue;
        }

        // start order parsing
        if !parsing {
            if line == "[Order]" {
                parsing = true;
                current_rule_type = Some(ERule::Order);

                continue;
            } else if line == "[Note]" {
                parsing = true;
                current_rule_type = Some(ERule::Note);
                current_warning_rule = Some(RuleKind::Note(Note::default()));
                continue;
            } else if line == "[Conflict]" {
                parsing = true;
                current_rule_type = Some(ERule::Conflict);
                continue;
            } else if line == "[Requires]" {
                parsing = true;
                current_rule_type = Some(ERule::Requires);
                continue;
            }
        }

        // parse current rule
        if parsing {
            if let Some(current_rule) = &current_rule_type {
                match current_rule {
                    ERule::Order => {
                        // order is just a list of names
                        current_order.push(line)
                    }
                    ERule::Note => {
                        // parse rule
                        todo!()
                    }
                    ERule::Conflict => todo!(),
                    ERule::Requires => todo!(),
                }
            }
        }
    }
    orders.push(current_order.to_owned());

    // process orders
    for o in orders {
        match o.len().cmp(&2) {
            std::cmp::Ordering::Less => continue,
            std::cmp::Ordering::Equal => order.push((o[0].to_owned(), o[1].to_owned())),
            std::cmp::Ordering::Greater => {
                // add all pairs
                for i in 0..o.len() - 1 {
                    order.push((o[i].to_owned(), o[i + 1].to_owned()));
                }
            }
        }
    }

    // set data
    rules.order = order;
    Ok(rules)
}

pub fn get_mods_from_rules(rules: &Rules) -> Vec<String> {
    let mut result: Vec<String> = vec![];
    for r in rules.order.iter() {
        let mut a = r.0.to_owned();
        if !result.contains(&a) {
            result.push(a);
        }
        a = r.1.to_owned();
        if !result.contains(&a) {
            result.push(a);
        }
    }
    result
}

pub fn gather_mods<P>(root: &P) -> io::Result<Vec<String>>
where
    P: AsRef<Path>,
{
    // gather mods from archive/pc/mod
    let archive_path = root.as_ref().join("archive").join("pc").join("mod");
    let mut entries = fs::read_dir(archive_path)?
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

    // TODO gather REDmods from mods/<NAME>
    entries.sort();

    Ok(entries)
}

////////////////////////////////////////////////////////////////////////
/// HELPERS
////////////////////////////////////////////////////////////////////////

// Returns an Iterator to the Reader of the lines of the file.
pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

// read file line by line into vector
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
