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
/// Panics if TODO
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
    // TODO parallelize
    let mut chunk: Option<Vec<u8>> = None;
    for line in reader.lines().flatten() {
        if chunk.is_some() && line.is_empty() {
            // end chunk
            if let Some(chunk) = chunk.take() {
                let chunk_reader = BufReader::new(chunk.as_slice());
                if let Some(r) = parse_chunk(chunk_reader) {
                    rules.extend(r);
                }
            }
        } else {
            // read to chunk
            if let Some(chunk) = &mut chunk {
                chunk.extend(line.as_bytes());
            } else {
                chunk = Some(line.as_bytes().to_vec());
            }
        }
    }

    Ok(rules)
}

/// Parses on rule section. Note: Order rules are returned as vec
///
/// # Panics
///
/// Panics if .
fn parse_chunk<R>(reader: R) -> Option<Vec<Rule>>
where
    R: BufRead,
{
    let mut current_rule: Option<Rule> = None;

    // pre-parse rule name
    // read the rest into a buffer
    let mut buffer: Vec<String> = vec![];
    let mut read_buffer = false;
    for line in reader.lines().flatten() {
        let mut line = line.trim().to_owned();

        // if the rule type has already been parsed we just copy the rest into a buffer for further parsing
        if read_buffer {
            buffer.push(line);
            continue;
        }

        // ignore comments
        if line.starts_with(';') {
            continue;
        }

        // Read the first non-comment line and expect a rule type

        if line.starts_with("[Order]") {
            // Order lines don't have in-line options
            current_rule = Some(Rule::Order(Order::default()));
            continue;
        } else if line.starts_with("[Note") {
            current_rule = Some(Rule::Note(Note::default()));
            line = line["[Note".len()..].to_owned();
        } else if line.starts_with("[Conflict") {
            current_rule = Some(Rule::Conflict(Conflict::default()));
            line = line["[Conflict".len()..].to_owned();
        } else if line.starts_with("[Requires") {
            current_rule = Some(Rule::Requires(Requires::default()));
            line = line["[Requires".len()..].to_owned();
        } else {
            // TODO unknown rule
            panic!("Parsing error: unknown rule");
        }

        // optional comment parser for rules of type [Note message] <body>
        line = line.trim().to_owned();
        let mut braket_cnt = 1;
        let mut comment = "".to_owned();
        for c in line.chars() {
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

        // rest of the line if anything
        line = line[&comment.len() + 1..].to_owned();
        line = line.trim().to_owned();
        if let Some(rule) = current_rule.as_mut() {
            rule.set_comment(comment);
        }
        if !line.is_empty() {
            buffer.push(line);
        }
        read_buffer = true;
    }

    // now parse rule body
    // TODO make these methods of the rules
    if let Some(rule) = current_rule {
        return match rule {
            Rule::Order(o) => parse_order_rule_body(o, buffer),
            Rule::Note(n) => parse_note_rule_body(n, buffer),
            Rule::Conflict(c) => parse_conflict_rule_body(c, buffer),
            Rule::Requires(r) => parse_requires_rule_body(r, buffer),
        };
    }

    None
}

fn parse_requires_rule_body(r: Requires, buffer: Vec<String>) -> Option<Vec<Rule>> {
    todo!()
}

fn parse_conflict_rule_body(c: Conflict, buffer: Vec<String>) -> Option<Vec<Rule>> {
    todo!()
}

fn parse_note_rule_body(n: Note, buffer: Vec<String>) -> Option<Vec<Rule>> {
    todo!()
}

/// Parse an order rule, it can have up to N items
fn parse_order_rule_body(current_rule: Order, buffer: Vec<String>) -> Option<Vec<Rule>> {
    let mut orders: Vec<Vec<String>> = vec![];
    let mut current_order: Vec<String> = vec![];

    // a b c d
    // parse each line
    for line in buffer {
        let r_line = line.trim().to_owned();

        // ignore comments
        if r_line.starts_with(';') {
            continue;
        }

        // HANDLE RULE PARSE
        // each line gets tokenized
        for token in tokenize(r_line) {
            current_order.push(token);
        }
    }
    orders.push(current_order.to_owned());

    // process order rules
    let mut rules: Vec<Rule> = vec![];
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

    Some(rules)
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
