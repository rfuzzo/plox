////////////////////////////////////////////////////////////////////////
/// PARSER
////////////////////////////////////////////////////////////////////////
use std::cmp::Ordering;

use std::fs::File;
use std::io::{self, Read};
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::rules::*;

/// Parse rules from a rules file
///
/// # Errors
///
/// This function will return an error if file io or parsing fails
pub fn parse_rules_from_path<P>(path: P) -> io::Result<Vec<Rule>>
where
    P: AsRef<Path>,
{
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let rules = parse_rules_from_reader(reader)?;
    Ok(rules)
}

/// Parse rules from a reader
///
/// # Panics
///
/// Panics if .
///
/// # Errors
///
/// This function will return an error if .
pub fn parse_rules_from_reader<R>(reader: R) -> io::Result<Vec<Rule>>
where
    R: Read + BufRead,
{
    let mut rules: Vec<Rule> = vec![];

    // pre-parse into rule blocks
    let mut chunk: Option<&[u8]> = None;

    for line in reader.lines().flatten() {
        // end chunk
        if chunk.is_some() && line.is_empty() {
            if let Some(chunk) = chunk.take() {
                let chunk_reader = BufReader::new(chunk);
                for rule in parse_chunk(chunk_reader) {
                    rules.push(rule);
                }
            }
        } else {
            // read
        }
    }

    Ok(rules)
}

/// Parses on rule section. Note: Order rules are returned as vec
///
/// # Panics
///
/// Panics if TODO
fn parse_chunk<R>(reader: R) -> Vec<Rule>
where
    R: BufRead,
{
    let mut rules: Vec<Rule> = vec![];

    // helpers for order rule
    let mut orders: Vec<Vec<String>> = vec![];
    let mut current_order: Vec<String> = vec![];

    // todo scan directory for user files

    let mut parsing = false;
    let mut current_rule: Option<Rule> = None;
    // parse each line
    for line in reader.lines().flatten() {
        // comments
        if line.starts_with(';') {
            continue;
        }

        // HANDLE RULE END
        // new empty lines end a rule block
        if parsing && line.is_empty() {
            //parsing = false;
            if let Some(rule) = current_rule.take() {
                // Order rule is handled separately
                if let Rule::Order(_o) = rule {
                    orders.push(current_order.to_owned());
                    current_order.clear();
                } else {
                    rules.push(rule);
                }
            } else {
                // error and abort
                panic!("Parsing error: unknown empty new line");
            }
            // TODO check end of chunk and warn if not reached
            break;
        }

        // HANDLE RULE START
        // start order parsing
        let mut r_line = line.trim().to_owned();
        if !parsing {
            // Order lines don't have in-line options
            if r_line.starts_with("[Order]") {
                current_rule = Some(Rule::Order(Order::default()));
                //r_line = r_line["[Order]".len()..].to_owned();
                continue;
            } else if r_line.starts_with("[Note") {
                current_rule = Some(Rule::Note(Note::default()));
                r_line = r_line["[Note".len()..].to_owned();
            } else if r_line.starts_with("[Conflict") {
                current_rule = Some(Rule::Conflict(Conflict::default()));
                r_line = r_line["[Conflict".len()..].to_owned();
            } else if r_line.starts_with("[Requires") {
                current_rule = Some(Rule::Requires(Requires::default()));
                r_line = r_line["[Requires".len()..].to_owned();
            } else {
                // unknown rule
                panic!("Parsing error: unknown rule");
            }

            // comment parser

            r_line = r_line.trim().to_owned();
            let mut braket_cnt = 1;
            let mut comment = "".to_owned();
            for c in r_line.chars() {
                if c == '[' {
                    braket_cnt += 1;
                } else if c == ']' {
                    braket_cnt -= 1;
                }
                if braket_cnt == 0 {
                    // we reached the end
                    break;
                }
                comment += c.to_string().as_str();
            }

            // rest of the line
            r_line = r_line[&comment.len() + 1..].to_owned();
            r_line = r_line.trim().to_owned();
            if let Some(rule) = current_rule.as_mut() {
                rule.set_comment(comment);
            }

            parsing = true;
        }

        // HANDLE RULE PARSE
        // parse current rule
        if parsing {
            if let Some(current_rule) = &current_rule {
                match current_rule {
                    Rule::Order(_o) => {
                        // tokenize
                        for token in tokenize(r_line) {
                            current_order.push(token);
                        }
                    }
                    Rule::Note(_n) => {
                        // parse rule
                        // Syntax: [Note optional-message] expr-1 expr-2 ... expr-N
                        // TODO alternative:
                        // [Note]
                        //  message
                        // A.esp

                        // subsequent lines are archive names

                        // parse expressions
                        todo!()
                    }
                    Rule::Conflict(_c) => {
                        todo!()
                    }
                    Rule::Requires(_r) => {
                        todo!()
                    }
                }
            }
        }
    }
    orders.push(current_order.to_owned());

    // process order rules
    for o in orders {
        match o.len().cmp(&2) {
            Ordering::Less => continue,
            Ordering::Equal => {
                rules.push(Rule::Order(Order::new(o[0].to_owned(), o[1].to_owned())))
            }
            Ordering::Greater => {
                // add all pairs
                for i in 0..o.len() - 1 {
                    rules.push(Rule::Order(Order::new(
                        o[i].to_owned(),
                        o[i + 1].to_owned(),
                    )));
                }
            }
        }
    }

    rules
}

/// Splits a String into string tokens (either separated but whitespace or wrapped in quotation marks)
pub fn tokenize(line: String) -> Vec<String> {
    let mut tokens: Vec<String> = vec![];

    let mut is_quoted = false;
    let mut current_token: String = "".to_owned();
    for c in line.chars() {
        if c == '"' {
            // started a quoted segment
            if is_quoted {
                is_quoted = false;
                // end token
                tokens.push(current_token.to_owned());
                current_token.clear();
            } else {
                is_quoted = true;
            }
        } else if c == ' ' {
            // end token
            tokens.push(current_token.to_owned());
            current_token.clear();
        } else {
            // read into token
            current_token += c.to_string().as_str();
        }
    }

    tokens
}
