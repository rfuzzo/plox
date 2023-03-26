use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufRead;
use std::io::{self};
use std::path::Path;
use toposort_scc::IndexGraph;

#[derive(Default)]
pub struct Rules {
    pub order: Vec<(String, String)>,
}

////////////////////////////////////////////////////////////////////////
/// RULES
////////////////////////////////////////////////////////////////////////

// todo replace with pattern matching
#[derive(Default)]
pub enum RuleKind {
    #[default]
    None,
    Order,

    Note,
    Conflict,
}

/// A rule as specified in the rules document
pub trait Rule {
    /// todo replace with pattern matching
    fn get_kind(&self) -> RuleKind;
    // every rule may have a comment describing why it failed
    fn get_comment(&self) -> &str;
    // every rule may be evaluated
    fn eval(&self, items: &[String]) -> bool;
}

/// A Note Rule
/// Notes simply check the expression and notify the user if eval is true
pub struct Note {
    pub comment: String,
    pub expression: Box<dyn Expression>,
}
impl Rule for Note {
    fn get_kind(&self) -> RuleKind {
        RuleKind::Note
    }
    fn get_comment(&self) -> &str {
        &self.comment
    }
    /// Notes evaluate as true if the expression evaluates as true
    fn eval(&self, items: &[String]) -> bool {
        self.expression.eval(items)
    }
}

/// A Conflict Rule
/// Conflicts evaluate as true if both expressions evaluate as true
pub struct Conflict {
    pub comment: String,
    // todo: make first atomic?
    pub expression_a: Box<dyn Expression>,
    pub expression_b: Box<dyn Expression>,
}
impl Rule for Conflict {
    fn get_kind(&self) -> RuleKind {
        RuleKind::Conflict
    }
    fn get_comment(&self) -> &str {
        &self.comment
    }
    /// Conflicts evaluate as true if both expressions evaluate as true
    fn eval(&self, items: &[String]) -> bool {
        self.expression_a.eval(items) && self.expression_b.eval(items)
    }
}

////////////////////////////////////////////////////////////////////////
/// EXPRESSIONS
////////////////////////////////////////////////////////////////////////

/// An expression such as EXISTS, AND, ANY, NOT
pub trait Expression {
    fn eval(&self, items: &[String]) -> bool;
}

/// An atomic expression (EXISTS)
/// atomics evaluate as true if the input list contains the item
pub struct Atomic {
    pub item: String,
}
impl Expression for Atomic {
    /// atomics evaluate as true if the input list contains the item
    fn eval(&self, items: &[String]) -> bool {
        // TODO wildcards
        items.contains(&self.item)
    }
}

/// An AND expression
/// AND evaluates as true if all expressions evaluate as true
pub struct AND {
    pub expressions: Vec<Box<dyn Expression>>,
}
impl Expression for AND {
    /// AND evaluates as true if all expressions evaluate as true
    fn eval(&self, items: &[String]) -> bool {
        let mut r = true;
        self.expressions
            .iter()
            .map(|e| e.eval(items))
            .for_each(|e| {
                r = r && e;
            });
        r
    }
}

/// An ANY expression
/// ANY evaluates as true if any expressions evaluates as true
pub struct ANY {
    pub expressions: Vec<Box<dyn Expression>>,
}
impl Expression for ANY {
    // ANY evaluate as true if any expressions evaluates as true
    fn eval(&self, items: &[String]) -> bool {
        let mut r = false;
        self.expressions
            .iter()
            .map(|e| e.eval(items))
            .for_each(|e| {
                r = r || e;
            });
        r
    }
}

/// A NOT expression
/// NOT evaluates as true if the wrapped expression evaluates as true
pub struct NOT {
    pub expression: Box<dyn Expression>,
}
impl Expression for NOT {
    // NOT evaluates as true if the wrapped expression evaluates as true
    fn eval(&self, items: &[String]) -> bool {
        !self.expression.eval(items)
    }
}

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
                let t = result[i].clone();
                result.remove(i);
                result.insert(j, t);
                // todo verbose
                //println!("[{x}-{y}] {result:?}");
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
    let mut result: Vec<String> = mods.clone().iter().map(|e| (*e).to_owned()).collect();
    println!("{result:?}");
    loop {
        if !stable_topo_sort_inner(mods.len(), &edges, &index_dict, &mut result) {
            break;
        }
    }

    // Return the sorted vector
    Ok(result)
}

// Returns an Iterator to the Reader of the lines of the file.
pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

// custom rules parser
pub fn parse_rules<P>(rules_dir: P) -> Result<Rules, &'static str>
where
    P: AsRef<Path>,
{
    let mut rules: Rules = Rules::default();
    let mut order: Vec<(String, String)> = vec![];

    let mut orders: Vec<Vec<String>> = vec![];

    // todo scan directory for user files
    let rules_path = rules_dir.as_ref().join("cmop_rules_base.txt");
    if let Ok(lines) = read_lines(rules_path) {
        let mut parsing = false;
        let mut current_order: Vec<String> = vec![];
        let mut current_rule: RuleKind = RuleKind::None;

        // Consumes the iterator, returns an (Optional) String
        for line in lines.flatten() {
            // parse each line
            if line.starts_with(';') {
                continue;
            }

            // order parsing
            if parsing && line.is_empty() {
                parsing = false;
                match current_rule {
                    RuleKind::Order => {
                        orders.push(current_order.to_owned());
                        current_order.clear();
                    }
                    RuleKind::None => todo!(),
                    RuleKind::Note => todo!(),
                    RuleKind::Conflict => todo!(),
                }

                continue;
            }

            if !parsing && line == "[Order]" {
                parsing = true;
                current_rule = RuleKind::Order;
                continue;
            }

            if parsing {
                match current_rule {
                    RuleKind::Order => current_order.push(line),
                    RuleKind::Note => todo!(),
                    RuleKind::None => todo!(),
                    RuleKind::Conflict => todo!(),
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
    } else {
        Err("failed reading file")
    }
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

pub fn verify_rules() {}

pub fn gather_mods<P>(root: &P) -> io::Result<Vec<String>>
where
    P: AsRef<Path>,
{
    // gather mods from archive/pc/mod
    let archive_path = root.as_ref().join("archive").join("pc").join("mod");
    let mut entries = fs::read_dir(archive_path)?
        .map(|res| res.map(|e| e.path()))
        .filter_map(Result::ok)
        .filter(|e| !e.is_dir())
        .filter(|e| e.extension().is_some())
        .filter(|e| e.extension().unwrap().to_str().unwrap().contains("archive"))
        .map(|e| String::from(e.file_name().unwrap().to_str().unwrap()))
        .collect::<Vec<_>>();

    // gather mods from mods/<NAME>

    entries.sort();

    Ok(entries)
}
