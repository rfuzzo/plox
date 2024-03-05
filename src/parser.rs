////////////////////////////////////////////////////////////////////////
/// PARSER
////////////////////////////////////////////////////////////////////////
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Error, ErrorKind, Read, Result, Seek, SeekFrom};
use std::path::Path;

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
                            ERule::Rule(w) => {
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

            // TODO lowercase all
            let line = line.to_lowercase();

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
                    rules.extend(it);
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
    fn parse_chunk<R>(&self, mut reader: R) -> Result<Vec<ERule>>
    where
        R: Read + BufRead + Seek,
    {
        // read first char
        let start = reader.read_u8()? as char;
        match start {
            '[' => {
                // start parsing
                let mut buf = vec![];
                // read until the end of the rule expression: e.g. [NOTE comment] body
                let _ = reader.read_until(b']', &mut buf)?;
                if let Ok(rule_expression) = String::from_utf8(buf[..buf.len() - 1].to_vec()) {
                    let rule: ERule;
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
                    let mut body = String::new();
                    for line in reader.lines().map_while(Result::ok).map(|f| {
                        if let Some(index) = f.find(';') {
                            f[..index].to_owned()
                        } else {
                            f.to_owned() // Return the entire string if ';' is not found
                        }
                    }) {
                        body += format!("{}\n", line).as_str();
                    }

                    let mut body = body.trim_start_matches('\n');
                    body = body.trim();
                    let body_cursor = Cursor::new(body);

                    // now parse rule body
                    match rule {
                        ERule::EOrderRule(o) => match o {
                            EOrderRule::Order(_) => self.parse_order_rule(body_cursor),
                            EOrderRule::NearStart(_) => {
                                let names = self.parse_near_rule(body_cursor)?;
                                Ok(vec![NearStart::new(names).into()])
                            }
                            EOrderRule::NearEnd(_) => {
                                let names = self.parse_near_rule(body_cursor)?;
                                Ok(vec![NearEnd::new(names).into()])
                            }
                        },
                        ERule::Rule(mut x) => {
                            EWarningRule::parse(&mut x, body_cursor, self)?;
                            Ok(vec![x.into()])
                        }
                    }
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

    fn parse_near_rule<R>(&self, reader: R) -> Result<Vec<String>>
    where
        R: Read + BufRead,
    {
        // parse each line
        let mut names: Vec<String> = vec![];
        for line in reader
            .lines()
            .map_while(Result::ok)
            .map(|l| l.trim().to_owned())
        {
            // HANDLE RULE PARSE
            // each line gets tokenized
            for token in self.tokenize(line) {
                if !self.ends_with_vec3(&token) {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Parsing error: tokenize failed",
                    ));
                }
                names.push(token);
            }
        }

        Ok(names)
    }

    /// .Parse an order rule, it can have up to N items
    ///
    /// # Errors
    ///
    /// This function will return an error if Order rule is missformed
    fn parse_order_rule<R>(&self, reader: R) -> Result<Vec<ERule>>
    where
        R: Read + BufRead,
    {
        // parse each line
        let mut order: Vec<String> = vec![];
        for line in reader
            .lines()
            .map_while(Result::ok)
            .map(|l| l.trim().to_owned())
        {
            // HANDLE RULE PARSE
            // each line gets tokenized
            for token in self.tokenize(line) {
                if !self.ends_with_vec3(&token) {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Parsing error: tokenize failed",
                    ));
                }
                order.push(token);
            }
        }

        // process order rules
        let mut rules: Vec<ERule> = vec![];
        match order.len().cmp(&2) {
            Ordering::Less => {
                // Rule with only one element is an error
                return Err(Error::new(
                    ErrorKind::Other,
                    "Logic error: order rule with less than two elements",
                ));
            }
            Ordering::Equal => rules.push(
                EOrderRule::Order(Order::new(order[0].to_owned(), order[1].to_owned())).into(),
            ),
            Ordering::Greater => {
                // add all pairs
                for i in 0..order.len() - 1 {
                    rules.push(
                        EOrderRule::Order(Order::new(order[i].to_owned(), order[i + 1].to_owned()))
                            .into(),
                    );
                }
            }
        }

        Ok(rules)
    }

    // TODO Clean up this shit :D
    pub fn ends_with_vec3(&self, current_buffer: &str) -> bool {
        let mut b = false;
        for ext in &self.ext {
            if current_buffer.to_lowercase().ends_with(ext) {
                b = true;
                break;
            }
        }

        b
    }

    fn ends_with_vec(&self, current_buffer: &str) -> bool {
        let mut b = false;
        for ext in &self.ext {
            if current_buffer
                .to_lowercase()
                .ends_with(format!("{} ", ext).as_str())
            {
                b = true;
                break;
            }
        }

        b
    }

    fn ends_with_vec2(&self, current_buffer: &str) -> bool {
        let mut b = false;
        for ext in &self.ext {
            if current_buffer
                .to_lowercase()
                .ends_with(format!("{} ", ext).as_str())
                || current_buffer
                    .to_lowercase()
                    .ends_with(format!("{}\n", ext).as_str())
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
            } else {
                // read into token
                current_token += c.to_string().as_str();
            }

            // check if we found an end
            if self.ends_with_vec(&current_token) {
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
    pub fn parse_expressions<R: Read + BufRead>(&self, mut reader: R) -> Result<Vec<Expression>> {
        let mut buffer = vec![];
        reader.read_to_end(&mut buffer)?;

        // pre-parse expressions into chunks
        let mut buffers: Vec<String> = vec![];
        let mut current_buffer: String = String::new();
        let mut is_expr = false;
        let mut is_token = false;
        let mut cnt = 0;

        for b in buffer {
            if is_expr {
                // if parsing an expression, just count brackets and read the rest into the buffer
                if b == b'[' {
                    cnt += 1;
                } else if b == b']' {
                    cnt -= 1;
                }
                current_buffer += &(b as char).to_string();

                if cnt == 0 {
                    // we reached the end of the current expression
                    is_expr = false;
                    buffers.push(current_buffer.to_owned());
                    current_buffer.clear();
                }
            } else if is_token {
                // if parsing tokens, check when ".archive" was parsed into the buffer and end
                current_buffer += &(b as char).to_string();

                if self.ends_with_vec2(&current_buffer) {
                    is_token = false;
                    buffers.push(current_buffer[..current_buffer.len() - 1].to_owned());
                    current_buffer.clear();
                }
            } else {
                // this marks the beginning
                if b == b'[' {
                    // start an expression
                    is_expr = true;
                    cnt += 1;
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
            buffers.push(current_buffer.to_owned());
            current_buffer.clear();
        }

        buffers = buffers
            .iter()
            .map(|f| f.trim().to_owned())
            .filter(|p| !p.is_empty())
            .collect();

        let mut expressions: Vec<Expression> = vec![];
        for buffer in buffers {
            match self.parse_expression(buffer.as_str()) {
                Ok(it) => {
                    expressions.push(it);
                }
                Err(err) => return Err(err),
            };
        }

        Ok(expressions)
    }

    /// Parses a single expression from a buffer
    ///
    /// # Errors
    ///
    /// This function will return an error if parsing fails
    pub fn parse_expression(&self, reader: &str) -> Result<Expression> {
        // an expression may start with
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
                // [DESC /regex/ A.esp] or // [DESC !/regex/ A.esp]
                let body = rest[..rest.len() - 1].trim_start();
                if let Some((regex, expr)) = Self::parse_desc_input(body) {
                    // do something
                    let expressions = self.parse_expressions(expr.as_bytes())?;
                    if let Some(first) = expressions.into_iter().last() {
                        let expr = DESC::new(first, regex);
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
            // is a token
            // in this case just return an atomic
            Ok(Atomic::from(reader).into())
        }
    }

    fn parse_desc_input(input: &str) -> Option<(String, String)> {
        if let Some(start_index) = input.find('/') {
            if let Some(end_index) = input.rfind('/') {
                // Extract the substring between "/" and "/"
                let left_part = input[start_index + 1..end_index].trim().to_string();

                // Extract the substring right of the last "/"
                let right_part = input[end_index + 1..].trim().to_string();

                // TODO fix negation
                return Some((left_part, right_part));
            }
        }

        if let Some(start_index) = input.find("!/") {
            if let Some(end_index) = input.rfind('/') {
                // Extract the substring between "/" and "/"
                let left_part = input[start_index + 2..end_index].trim().to_string();

                // Extract the substring right of the last "/"
                let right_part = input[end_index + 1..].trim().to_string();

                return Some((left_part, right_part));
            }
        }
        None
    }
}

/// .Reads the first comment lines of a rule chunk and returns the rest as byte buffer
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
