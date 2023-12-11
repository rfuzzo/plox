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
                if let Some(r) = pre_parse_chunk(chunk_reader) {
                    rules.extend(r);
                }
            }
        } else {
            // read to chunk
            if let Some(chunk) = &mut chunk {
                chunk.extend(line.as_bytes());
                chunk.push(b'\n');
            } else {
                let delimited_line = line + "\n";
                chunk = Some(delimited_line.as_bytes().to_vec());
            }
        }
    }
    // parse last chunk
    if let Some(chunk) = chunk.take() {
        let chunk_reader = BufReader::new(chunk.as_slice());
        if let Some(r) = pre_parse_chunk(chunk_reader) {
            rules.extend(r);
        }
    }

    Ok(rules)
}

/// Parses on rule section. Note: Order rules are returned as vec
///
/// # Panics
///
/// Panics if .
fn pre_parse_chunk<R>(reader: R) -> Option<Vec<Rule>>
where
    R: BufRead,
{
    let mut current_rule: Option<Rule> = None;

    // pre-parse rule name
    // read the rest into a buffer
    let mut lines: Vec<String> = vec![];
    let mut read_buffer = false;
    for line in reader.lines().flatten() {
        let mut line = line.to_owned();
        // if the rule type has already been parsed we just copy the rest into a buffer for further parsing
        if read_buffer {
            lines.push(line);
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
            read_buffer = true;
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
            lines.push(line);
        }
        read_buffer = true;
    }

    // now parse rule body
    // TODO make these methods of the rules
    if let Some(rule) = current_rule {
        match rule {
            // Order rules don't have comments and no expressions so we can just parse them individually
            Rule::Order(o) => return parse_order_rule(o, lines),
            mut x => {
                // pre-parse comments
                let buffer = pre_parse_comment(&mut x, lines);
                x.parse(buffer);
                return Some(vec![x]);
            }
        };
    }

    None
}

/// Reads the first comment lines of a rule chunk and returns the rest as byte buffer
fn pre_parse_comment(rule: &mut Rule, body: Vec<String>) -> Vec<u8> {
    // the first lines starting with a whitespace may be comments
    let mut read_comment = false;
    let mut comment = "".to_owned();
    let mut is_first_line = true;
    let mut buffer: Vec<u8> = vec![];
    for line in body {
        // ignore comments
        if line.starts_with(';') {
            continue;
        }

        // handle rule comments
        if is_first_line && line.starts_with(' ') {
            read_comment = true;
            comment += line.trim_start();
            is_first_line = false;
            continue;
        }
        is_first_line = false;
        if read_comment {
            match line.starts_with(' ') {
                true => {
                    comment += line.trim_start();
                    continue;
                }
                false => {
                    read_comment = false;
                    buffer.extend(line.as_bytes());
                    buffer.push(b'\n');
                    continue; // pre-parsing finished, read the rest into a binary buffer
                }
            }
        } else {
            buffer.extend(line.as_bytes());
            buffer.push(b'\n');
        }
    }

    // TODO override inline comment with multi-line comment
    if !comment.is_empty() {
        rule.set_comment(comment);
    }

    buffer
}

/// Parse an order rule, it can have up to N items
fn parse_order_rule(_rule: Order, buffer: Vec<String>) -> Option<Vec<Rule>> {
    let mut order: Vec<String> = vec![];

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
            order.push(token);
        }
    }

    // process order rules
    let mut rules: Vec<Rule> = vec![];
    match order.len().cmp(&2) {
        Ordering::Less => {}
        Ordering::Equal => rules.push(Rule::Order(Order::new(
            order[0].to_owned(),
            order[1].to_owned(),
        ))),
        Ordering::Greater => {
            // add all pairs
            for i in 0..order.len() - 1 {
                rules.push(Rule::Order(Order::new(
                    order[i].to_owned(),
                    order[i + 1].to_owned(),
                )));
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
            // ignore whitespace in quoted segments
            if !is_quoted {
                // end token
                if !current_token.is_empty() {
                    tokens.push(current_token.to_owned());
                    current_token.clear();
                }
            } else {
                current_token += c.to_string().as_str();
            }
        } else {
            // read into token
            current_token += c.to_string().as_str();
        }
    }

    if !current_token.is_empty() {
        tokens.push(current_token.to_owned());
    }

    tokens
}
