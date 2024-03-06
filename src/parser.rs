////////////////////////////////////////////////////////////////////////

use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Error, ErrorKind, Read, Result, Seek, SeekFrom};
use std::path::Path;
use std::usize;

use byteorder::ReadBytesExt;
use log::*;

use crate::{expressions::*, TParser};
use crate::{rules::*, ESupportedGame};

pub struct Parser {
    pub game: ESupportedGame,
    pub ext: Vec<String>,

    pub order_rules: Vec<EOrderRule>,
    pub rules: Vec<EWarningRule>,
}

pub fn get_parser(game: ESupportedGame) -> Parser {
    match game {
        ESupportedGame::Morrowind => new_tes3_parser(),
        ESupportedGame::OpenMorrowind => new_openmw_parser(),
        ESupportedGame::Cyberpunk => new_cyberpunk_parser(),
    }
}

pub fn new_cyberpunk_parser() -> Parser {
    Parser::new(vec![".archive".into()], ESupportedGame::Cyberpunk)
}

pub fn new_tes3_parser() -> Parser {
    Parser::new(
        vec![".esp".into(), ".esm".into()],
        ESupportedGame::Morrowind,
    )
}

pub fn new_openmw_parser() -> Parser {
    Parser::new(
        vec![".esp".into(), ".esm".into(), ".omwaddon".into()],
        ESupportedGame::OpenMorrowind,
    )
}

#[derive(Debug)]
struct ChunkWrapper {
    data: Vec<u8>,
    info: String,
}

impl ChunkWrapper {
    fn new(data: Vec<u8>, info: String) -> Self {
        Self { data, info }
    }
}

impl Parser {
    pub fn new(ext: Vec<String>, game: ESupportedGame) -> Self {
        Self {
            ext,
            game,
            rules: vec![],
            order_rules: vec![],
        }
    }

    /// Parse rules for a specific game, expects the path to be the rules directory
    ///
    /// # Errors
    ///
    /// This function will return an error if file io or parsing fails
    pub fn init<P>(&mut self, path: P)
    where
        P: AsRef<Path>,
    {
        self.rules.clear();
        self.order_rules.clear();

        let rules_files = match self.game {
            ESupportedGame::Morrowind | ESupportedGame::OpenMorrowind => {
                ["mlox_base.txt", "mlox_user.txt", "mlox_my_rules.txt"].as_slice()
            }
            ESupportedGame::Cyberpunk => ["plox_base.txt", "plox_my_rules.txt"].as_slice(),
        };

        for file in rules_files {
            let path = path.as_ref().join(file);
            if path.exists() {
                if let Ok(rules) = self.parse_rules_from_path(&path) {
                    info!("Parsed file {} with {} rules", path.display(), rules.len());

                    for r in rules {
                        match r {
                            ERule::EOrderRule(o) => {
                                self.order_rules.push(o);
                            }
                            ERule::EWarningRule(w) => {
                                self.rules.push(w);
                            }
                        }
                    }
                }
            } else {
                warn!("Could not find rules file {}", path.display());
            }
        }

        info!("Parser initialized with {} rules", self.rules.len());
    }

    /// Parse rules from a rules file
    ///
    /// # Errors
    ///
    /// This function will return an error if file io or parsing fails
    pub fn parse_rules_from_path<P>(&self, path: P) -> Result<Vec<ERule>>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let rules = self.parse_rules_from_reader(reader)?;
        Ok(rules)
    }

    /// Parse rules from a reader
    ///
    /// # Errors
    ///
    /// This function will return an error if parsing fails
    pub fn parse_rules_from_reader<R>(&self, reader: R) -> Result<Vec<ERule>>
    where
        R: Read + BufRead + Seek,
    {
        // pre-parse into rule blocks
        let mut chunks: Vec<ChunkWrapper> = vec![];
        let mut chunk: Option<ChunkWrapper> = None;
        for (idx, line) in reader.lines().map_while(Result::ok).enumerate() {
            // ignore comments
            if line.trim_start().starts_with(';') {
                continue;
            }
            // lowercase all
            let mut line = line.to_lowercase();

            line = if let Some(index) = line.find(';') {
                line[..index].trim_end().to_owned()
            } else {
                line.trim_end().to_owned()
            };

            if chunk.is_some() && line.trim().is_empty() {
                // end chunk
                if let Some(chunk) = chunk.take() {
                    chunks.push(chunk);
                }
            } else if !line.trim().is_empty() {
                // read to chunk, preserving newline delimeters
                let delimited_line = line + "\n";
                if let Some(chunk) = &mut chunk {
                    chunk.data.extend(delimited_line.as_bytes());
                } else {
                    chunk = Some(ChunkWrapper::new(
                        delimited_line.as_bytes().to_vec(),
                        (idx + 1).to_string(),
                    ));
                }
            }
        }
        // parse last chunk
        if let Some(chunk) = chunk.take() {
            chunks.push(chunk);
        }

        // process chunks
        let mut rules: Vec<ERule> = vec![];
        for (idx, chunk) in chunks.into_iter().enumerate() {
            let info = &chunk.info;

            let cursor = Cursor::new(&chunk.data);
            match self.parse_chunk(cursor) {
                Ok(it) => {
                    rules.push(it);
                }
                Err(err) => {
                    // log error and skip chunk
                    debug!(
                        "Error '{}' at chunk #{}, starting at line: {}",
                        err, idx, info
                    );
                    let string = String::from_utf8(chunk.data).expect("not valid utf8");
                    debug!("{}", string);
                }
            };
        }

        Ok(rules)
    }

    /// Parses on rule section. Note: Order rules are returned as vec
    ///
    /// # Errors
    ///
    /// This function will return an error if parsing fails
    fn parse_chunk<R>(&self, mut reader: R) -> Result<ERule>
    where
        R: Read + BufRead + Seek,
    {
        // read first char
        let start = reader.read_u8()? as char;
        match start {
            '[' => {
                // start parsing
                // read until the end of the rule expression: e.g. [NOTE comment] body
                if let Ok(mut rule_expression) = parse_rule_expression(&mut reader) {
                    rule_expression.pop();
                    let mut rule: ERule;
                    // parse rule name
                    {
                        if rule_expression.strip_prefix("order").is_some() {
                            rule = Order::default().into();
                        } else if rule_expression.strip_prefix("nearstart").is_some() {
                            rule = NearStart::default().into();
                        } else if rule_expression.strip_prefix("nearend").is_some() {
                            rule = NearEnd::default().into();
                        } else if let Some(rest) = rule_expression.strip_prefix("note") {
                            let mut x = Note::default();
                            x.set_comment(rest.trim().to_owned());
                            rule = x.into();
                        } else if let Some(rest) = rule_expression.strip_prefix("conflict") {
                            let mut x = Conflict::default();
                            x.set_comment(rest.trim().to_owned());
                            rule = x.into();
                        } else if let Some(rest) = rule_expression.strip_prefix("requires") {
                            let mut x = Requires::default();
                            x.set_comment(rest.trim().to_owned());
                            rule = x.into();
                        } else if let Some(rest) = rule_expression.strip_prefix("patch") {
                            let mut x = Patch::default();
                            x.set_comment(rest.trim().to_owned());
                            rule = x.into();
                        } else {
                            // unknown rule, skip
                            return Err(Error::new(
                                ErrorKind::Other,
                                "Parsing error: unknown rule",
                            ));
                        }
                    }

                    // parse body

                    // construct the body out of each line with comments trimmed
                    let mut is_first_line = false;
                    let mut comment = String::new();
                    let mut body = String::new();
                    for (idx, line) in reader
                        .lines()
                        .map_while(Result::ok)
                        .map(|f| {
                            if let Some(index) = f.find(';') {
                                f[..index].to_owned()
                            } else {
                                f.to_owned() // Return the entire string if ';' is not found
                            }
                        })
                        .filter(|p| !p.trim().is_empty())
                        .enumerate()
                    {
                        if idx == 0 {
                            is_first_line = true;
                        }

                        // check for those darned comments
                        if is_first_line {
                            if let Some(first_char) = line.chars().next() {
                                if first_char.is_ascii_whitespace() {
                                    comment += line.as_str();
                                    continue;
                                }
                            }

                            if !comment.is_empty() {
                                if let ERule::EWarningRule(w) = &mut rule {
                                    w.set_comment(comment.clone().trim().into());
                                }
                                comment.clear();
                            }

                            is_first_line = false;
                        }

                        // this is a proper line
                        body += format!("{}\n", line).as_str();
                    }

                    let body = body.trim();
                    let body_cursor = Cursor::new(body);

                    // now parse rule body
                    ERule::parse(&mut rule, body_cursor, self)?;
                    Ok(rule)
                } else {
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

    pub fn ends_with_vec(&self, current_buffer: &str) -> bool {
        let mut b = false;
        for ext in &self.ext {
            if current_buffer.ends_with(ext) {
                b = true;
                break;
            }
        }

        b
    }
    fn ends_with_vec_whitespace(&self, current_buffer: &str) -> bool {
        let mut b = false;
        for ext in &self.ext {
            if current_buffer.ends_with(format!("{} ", ext).as_str()) {
                b = true;
                break;
            }
        }

        b
    }
    fn ends_with_vec2_whitespace_or_newline(&self, current_buffer: &str) -> bool {
        let mut b = false;
        for ext in &self.ext {
            if current_buffer.ends_with(format!("{} ", ext).as_str())
                || current_buffer.ends_with(format!("{}\n", ext).as_str())
            {
                b = true;
                break;
            }
        }

        b
    }

    /// Splits a String into string tokens (either separated by extension or wrapped in quotation marks)
    pub fn tokenize(&self, line: String) -> Vec<String> {
        let mut tokens: Vec<String> = vec![];

        // ignore everything after ;
        let mut line = line.clone();
        if line.contains(';') {
            line = line.split(';').next().unwrap_or("").trim().to_owned();
        }

        let mut is_quoted = false;
        let mut current_token: String = "".to_owned();
        for c in line.chars() {
            // check quoted and read in chars
            if c == '"' {
                // started a quoted segment
                if is_quoted {
                    is_quoted = false;
                    // end token
                    tokens.push(current_token.trim().to_owned());
                    current_token.clear();
                } else {
                    is_quoted = true;
                }
                continue;
            }
            current_token += c.to_string().as_str();

            // check if we found an end
            if self.ends_with_vec_whitespace(&current_token) {
                // ignore whitespace in quoted segments
                if !is_quoted {
                    // end token
                    if !current_token.is_empty() {
                        tokens.push(current_token.trim().to_owned());
                        current_token.clear();
                    }
                } else {
                    current_token += c.to_string().as_str();
                }
            }
        }

        if !current_token.is_empty() {
            tokens.push(current_token.trim().to_owned());
        }

        tokens
    }

    /// Parses all expressions from a buffer until EOF is reached
    ///
    /// # Errors
    ///
    /// This function will return an error if parsing fails anywhere
    pub fn parse_expressions<R>(&self, mut reader: R) -> Result<Vec<Expression>>
    where
        R: Read + BufRead,
    {
        let mut buffer = vec![];
        reader.read_to_end(&mut buffer)?;

        // pre-parse expressions into chunks
        let mut chunks: Vec<(String, bool)> = vec![];
        let mut current_buffer: String = String::new();
        let mut is_expr = false;
        let mut is_token = false;
        let mut depth = 0;

        for b in buffer {
            if is_expr {
                // if parsing an expression, just count brackets and read the rest into the buffer
                if b == b'[' {
                    depth += 1;
                } else if b == b']' {
                    depth -= 1;
                }
                current_buffer += &(b as char).to_string();

                // check if really an expression
                // valid expressions are [ANY], [ALL], [NOT], [DESC], [SIZE], [VER]

                if depth == 0 {
                    // we reached the end of the current expression
                    let trimmed = current_buffer.trim();
                    if starts_with_whitespace(trimmed, "[any")
                        || starts_with_whitespace(trimmed, "[all")
                        || starts_with_whitespace(trimmed, "[not")
                        || starts_with_whitespace(trimmed, "[desc")
                        || starts_with_whitespace(trimmed, "[size")
                        || starts_with_whitespace(trimmed, "[ver")
                    {
                        is_expr = false;
                        chunks.push((trimmed.to_owned(), true));

                        current_buffer.clear();
                    } else {
                        // not a valid expression
                        // move into token
                        is_expr = false;
                        is_token = true;
                    }
                }
            } else if is_token {
                // if parsing tokens, check when ".archive" was parsed into the buffer and end
                current_buffer += &(b as char).to_string();

                if self.ends_with_vec2_whitespace_or_newline(&current_buffer) {
                    is_token = false;
                    chunks.push((current_buffer[..current_buffer.len() - 1].to_owned(), false));
                    current_buffer.clear();
                }
            } else {
                // this marks the beginning
                if b == b'[' {
                    // start an expression
                    is_expr = true;
                    depth += 1;
                }
                // ignore whitespace
                else if !b.is_ascii_whitespace() {
                    is_token = true;
                }
                current_buffer += &(b as char).to_string();
            }
        }

        // rest
        if !current_buffer.is_empty() {
            chunks.push((current_buffer.to_owned(), is_expr));
            current_buffer.clear();
        }

        chunks = chunks
            .iter()
            .map(|f| (f.0.trim().to_owned(), f.1))
            .filter(|p| !p.0.is_empty())
            .collect();

        let mut expressions: Vec<Expression> = vec![];
        for (chunk, is_expr) in chunks {
            match self.parse_expression(chunk.as_str(), is_expr) {
                Ok(it) => {
                    expressions.push(it);
                }
                Err(err) => return Err(err),
            }
        }

        Ok(expressions)
    }

    /// Parses a single expression from a buffer
    ///
    /// # Errors
    ///
    /// This function will return an error if parsing fails
    pub fn parse_expression(&self, reader: &str, is_expression: bool) -> Result<Expression> {
        // an expression may start with
        if !is_expression {
            // is a token
            // in this case just return an atomic
            if !self.ends_with_vec(reader) {
                return Err(Error::new(ErrorKind::Other, "Parsing error: Not an atomic"));
            }

            return Ok(Atomic::from(reader).into());
        }

        if reader.starts_with('[') {
            // is an expression
            // parse the kind and reurse down
            if let Some(rest) = reader.strip_prefix("[any") {
                let expressions =
                    self.parse_expressions(rest[..rest.len() - 1].trim_start().as_bytes())?;
                let expr = ANY::new(expressions);
                Ok(expr.into())
            } else if let Some(rest) = reader.strip_prefix("[all") {
                let expressions =
                    self.parse_expressions(rest[..rest.len() - 1].trim_start().as_bytes())?;
                let expr = ALL::new(expressions);
                Ok(expr.into())
            } else if let Some(rest) = reader.strip_prefix("[not") {
                let expressions =
                    self.parse_expressions(rest[..rest.len() - 1].trim_start().as_bytes())?;
                if let Some(first) = expressions.into_iter().last() {
                    let expr = NOT::new(first);
                    Ok(expr.into())
                } else {
                    Err(Error::new(
                        ErrorKind::Other,
                        "Parsing error: unknown expression",
                    ))
                }
            } else if let Some(rest) = reader.strip_prefix("[desc") {
                let body = rest[..rest.len() - 1].trim_start();
                if let Some((expr, regex, negated)) = parse_desc(body) {
                    // do something
                    let expressions = self.parse_expressions(expr.as_bytes())?;
                    if let Some(first) = expressions.into_iter().last() {
                        let expr = DESC::new(first, regex, negated);
                        return Ok(expr.into());
                    }
                }
                Err(Error::new(
                    ErrorKind::Other,
                    "Parsing error: unknown expression",
                ))
            } else if let Some(rest) = reader.strip_prefix("[size") {
                let body = rest[..rest.len() - 1].trim_start();
                if let Some((expr, size, negated)) = parse_size(body) {
                    // do something
                    let expressions = self.parse_expressions(expr.as_bytes())?;
                    if let Some(first) = expressions.into_iter().last() {
                        let expr = SIZE::new(first, size, negated);
                        return Ok(expr.into());
                    }
                }
                Err(Error::new(
                    ErrorKind::Other,
                    "Parsing error: unknown expression",
                ))
            } else if let Some(rest) = reader.strip_prefix("[ver") {
                let body = rest[..rest.len() - 1].trim_start();
                if let Some((expr, operator, version)) = parse_ver(body) {
                    // do something
                    let expressions = self.parse_expressions(expr.as_bytes())?;
                    if let Some(first) = expressions.into_iter().last() {
                        let expr = VER::new(first, operator, version);
                        return Ok(expr.into());
                    }
                }
                Err(Error::new(
                    ErrorKind::Other,
                    "Parsing error: unknown expression",
                ))
            } else {
                // unknown expression
                Err(Error::new(
                    ErrorKind::Other,
                    "Parsing error: unknown expression",
                ))
            }
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "Parsing error: Not an expression",
            ))
        }
    }
}

fn starts_with_whitespace(current_buffer: &str, arg: &str) -> bool {
    current_buffer.starts_with(format!("{} ", arg).as_str())
        || current_buffer.starts_with(format!("{}\t", arg).as_str())
}

/// Reads the first comment lines of a rule chunk and returns the rest as byte buffer
///
/// # Errors
///
/// This function will return an error if stream reading or seeking fails
pub fn read_comment<R: Read + BufRead + Seek>(reader: &mut R) -> Result<Option<String>> {
    // a line starting with a whitespace may be a comment
    let first_char = reader.read_u8()? as char;
    if first_char == ' ' || first_char == '\t' {
        // this is a comment
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let mut comment = line.trim().to_owned();

        if let Ok(Some(c)) = read_comment(reader) {
            comment += c.as_str();
        }

        Ok(Some(comment))
    } else {
        reader.seek(SeekFrom::Current(-1))?;
        Ok(None)
    }
}

fn parse_desc(input: &str) -> Option<(String, String, bool)> {
    //  !/Bite works only with Vampire Embrace/ DW_assassination.esp]
    if let Some(input) = input.strip_prefix("!/") {
        if let Some(end_index) = input.rfind('/') {
            // Extract the substring between "/" and "/"
            let left_part = input[..end_index].trim().to_string();

            // Extract the substring right of the last "/"
            let right_part = input[end_index + 1..].trim().to_string();

            return Some((right_part, left_part, true));
        }
    }
    //  /This version is compatible with Better Robes and Better Clothes./ UFR_v3dot2.esp]
    else if let Some(input) = input.strip_prefix('/') {
        if let Some(end_index) = input.rfind('/') {
            // Extract the substring between "/" and "/"
            let left_part = input[..end_index].trim().to_string();

            // Extract the substring right of the last "/"
            let right_part = input[end_index + 1..].trim().to_string();

            return Some((right_part, left_part, false));
        }
    }

    None
}

fn parse_size(input: &str) -> Option<(String, usize, bool)> {
    // !4921700 Annastia V3.3.esp]
    if let Some(input) = input.strip_prefix('!') {
        if let Some(left_part) = input.split_whitespace().next() {
            if let Some(right_part) = input.trim_start().strip_prefix(left_part) {
                if let Ok(size) = left_part.parse::<usize>() {
                    return Some((right_part[1..].to_owned(), size, true));
                }
            }
        }
    }
    // 591786 BMS_Timers_Patch.esp]
    else if let Some(left_part) = input.split_whitespace().next() {
        if let Some(right_part) = input.trim_start().strip_prefix(left_part) {
            if let Ok(size) = left_part.parse::<usize>() {
                return Some((right_part[1..].to_owned(), size, false));
            }
        }
    }

    None
}

fn parse_ver(input: &str) -> Option<(String, EVerOperator, String)> {
    // >1.51 Rise of House Telvanni.esm
    // = 2.14 Blood and Gore.esp
    // < 3.1 Class Abilities <VER>.esp
    if let Some(input) = input.strip_prefix('<') {
        if let Some(version) = input.split_whitespace().next() {
            if let Some(right_part) = input.trim_start().strip_prefix(version) {
                return Some((
                    right_part.to_owned(),
                    EVerOperator::Less,
                    version.to_owned(),
                ));
            }
        }
    } else if let Some(input) = input.strip_prefix('>') {
        if let Some(version) = input.split_whitespace().next() {
            if let Some(right_part) = input.trim_start().strip_prefix(version) {
                return Some((
                    right_part.to_owned(),
                    EVerOperator::Greater,
                    version.to_owned(),
                ));
            }
        }
    } else if let Some(input) = input.strip_prefix('=') {
        if let Some(version) = input.split_whitespace().next() {
            if let Some(right_part) = input.trim_start().strip_prefix(version) {
                return Some((
                    right_part.to_owned(),
                    EVerOperator::Equal,
                    version.to_owned(),
                ));
            }
        }
    }

    None
}

fn parse_rule_expression<R>(mut reader: R) -> Result<String>
where
    R: Read,
{
    let mut scope = 1;
    let mut buffer = Vec::new();
    let end_index;

    loop {
        let mut byte = [0; 1];
        match reader.read_exact(&mut byte) {
            Ok(_) => {
                buffer.push(byte[0]);
                if byte[0] == b'[' {
                    scope += 1;
                } else if byte[0] == b']' {
                    scope -= 1;
                    if scope == 0 {
                        end_index = buffer.len();
                        break;
                    }
                }
            }
            Err(err) => {
                eprintln!("Error reading input: {}", err);
                return Err(err);
            }
        }
    }

    buffer.truncate(end_index);
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_parse_rule_expression() -> Result<()> {
        {
            let inputs = [
                ("NOTE comment] more content.", "NOTE comment]"),
                ("NOTE] more content.", "NOTE]"),
                ("NOTE comment]", "NOTE comment]"),
                ("NOTE with [nested] comment]", "NOTE with [nested] comment]"),
                (
                    "NOTE with [nested] comment] and more",
                    "NOTE with [nested] comment]",
                ),
            ];

            for (input, expected) in inputs {
                assert_eq!(
                    expected.to_owned(),
                    parse_rule_expression(input.as_bytes())?
                );
            }
        }

        {
            let inputs = [
                ("NOTE comment[]"),
                ("NOTE comment[with] [[[[[broken scope]"),
            ];

            for input in inputs {
                assert!(parse_rule_expression(input.as_bytes()).is_err())
            }
        }

        Ok(())
    }

    #[test]
    fn test_tokenize() {
        let parser = new_cyberpunk_parser();

        let inputs = [
            vec!["a.archive", "my e3.archive.archive"],
            vec![" a.archive", "\"mod with spaces.archive\"", "b.archive"],
            vec![" a.archive", "\"mod with spaces.archive\"", "\"c.archive\""],
            vec!["a mod with spaces.archive"],
            vec!["a.archive"],
        ];

        for input_vec in inputs {
            let input = input_vec.join(" ");
            let expected = input_vec
                .iter()
                .map(|f| f.trim().trim_matches('"').trim())
                .collect::<Vec<_>>();
            assert_eq!(expected, parser.tokenize(input.to_owned()).as_slice());
        }
    }
}
