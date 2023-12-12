////////////////////////////////////////////////////////////////////////
/// PARSER
////////////////////////////////////////////////////////////////////////
use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Cursor, Error, ErrorKind, Read, Seek};
use std::path::Path;

use byteorder::ReadBytesExt;

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
    R: Read + BufRead + Seek,
{
    // pre-parse into rule blocks
    let mut chunks: Vec<Vec<u8>> = vec![];
    let mut chunk: Option<Vec<u8>> = None;
    for line in reader.lines().flatten() {
        // ignore comments
        if line.trim_start().starts_with(';') {
            continue;
        }

        if chunk.is_some() && line.trim().is_empty() {
            // end chunk
            if let Some(chunk) = chunk.take() {
                chunks.push(chunk);
            }
        } else {
            // read to chunk, preserving newline delimeters
            let delimited_line = line + "\n";
            if let Some(chunk) = &mut chunk {
                chunk.extend(delimited_line.as_bytes());
            } else {
                chunk = Some(delimited_line.as_bytes().to_vec());
            }
        }
    }
    // parse last chunk
    if let Some(chunk) = chunk.take() {
        chunks.push(chunk);
    }

    // process chunks
    // TODO parallelize
    let mut rules: Vec<Rule> = vec![];
    for chunk in chunks {
        let cursor = Cursor::new(chunk);
        let parsed = parse_chunk(cursor)?;
        rules.extend(parsed);
    }

    Ok(rules)
}

/// Parses on rule section. Note: Order rules are returned as vec
///
/// # Panics
///
/// Panics if .
fn parse_chunk<R>(mut reader: R) -> io::Result<Vec<Rule>>
where
    R: Read + BufRead + Seek,
{
    // read first char
    let start = reader.read_u8()? as char;
    match start {
        '[' => {
            // start parsing
            let mut buf = vec![];
            let _ = reader.read_until(b']', &mut buf)?;
            if let Ok(line) = String::from_utf8(buf[..buf.len() - 1].to_vec()) {
                // parse rule name
                let rule: Rule;
                if line.strip_prefix("Order").is_some() {
                    // Order lines don't have in-line options
                    rule = Order::default().into();
                } else if let Some(rest) = line.strip_prefix("Note") {
                    let mut x = Note::default();
                    x.set_comment(rest.trim().to_owned());
                    rule = x.into();
                } else if let Some(rest) = line.strip_prefix("Conflict") {
                    let mut x = Conflict::default();
                    x.set_comment(rest.trim().to_owned());
                    rule = x.into();
                } else if let Some(rest) = line.strip_prefix("Requires") {
                    let mut x = Requires::default();
                    x.set_comment(rest.trim().to_owned());
                    rule = x.into();
                } else {
                    // TODO unknown rule
                    return Err(Error::new(ErrorKind::Other, "Parsing error: unknown rule"));
                }

                // parse buffer
                // some ad-hoc fixes because we have inline-rules
                let mut lin = String::new();
                reader.read_line(&mut lin)?;
                lin = lin.trim_start().to_owned();

                if !lin.is_empty() {
                    // if the line is not empty we have an inline expression and we need to trim and read back to buffer
                    reader.seek(io::SeekFrom::Current(-(lin.len() as i64)))?;
                }

                let mut body = vec![];
                reader.read_to_end(&mut body)?;
                let body_cursor = Cursor::new(body);

                // now parse rule body
                match rule {
                    // Order rules don't have comments and no expressions so we can just parse them individually
                    Rule::Order(_) => parse_order_rule(body_cursor),
                    mut x => {
                        let r = x.parse(body_cursor)?;
                        Ok(vec![r])
                    }
                }
            } else {
                // TODO return
                Err(Error::new(ErrorKind::Other, "Parsing error: unknown rule"))
            }
        }
        _ => {
            // error
            Err(Error::new(
                ErrorKind::Other,
                "Parsing error: Not a rule start",
            ))
        }
    }
}

/// Reads the first comment lines of a rule chunk and returns the rest as byte buffer
pub fn read_comment<R: Read + BufRead + Seek>(reader: &mut R) -> io::Result<Option<String>> {
    // a line starting with a whitespace may be a comment
    if reader.read_u8()? as char != ' ' {
        reader.seek(io::SeekFrom::Current(-1))?;
        return Ok(None);
    }

    // this is a comment
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut comment = line.trim().to_owned();

    if let Ok(Some(c)) = read_comment(reader) {
        comment += c.as_str();
    }

    Ok(Some(comment))
}

/// Parse an order rule, it can have up to N items
fn parse_order_rule<R>(reader: R) -> io::Result<Vec<Rule>>
where
    R: Read + BufRead,
{
    let mut order: Vec<String> = vec![];

    // parse each line
    for line in reader.lines().flatten().map(|l| l.trim().to_owned()) {
        // HANDLE RULE PARSE
        // each line gets tokenized
        for token in tokenize(line) {
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

    Ok(rules)
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
