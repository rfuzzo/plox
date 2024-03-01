////////////////////////////////////////////////////////////////////////
/// PARSER
////////////////////////////////////////////////////////////////////////
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Error, ErrorKind, Read, Result, Seek, SeekFrom};
use std::path::Path;

use byteorder::ReadBytesExt;
use log::{debug, error, info, warn};

use crate::{expressions::*, TParser};
use crate::{rules::*, ESupportedGame};

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

pub struct Parser {
    pub game: ESupportedGame,
    pub ext: Vec<String>,

    pub rules: Vec<Rule>,
}

pub fn get_parser(game: ESupportedGame) -> Parser {
    match game {
        ESupportedGame::Morrowind => Parser::new_tes3_parser(),
        ESupportedGame::OpenMorrowind => Parser::new_openmw_parser(),
        ESupportedGame::Cyberpunk => Parser::new_cyberpunk_parser(),
    }
}

impl Parser {
    pub fn new(ext: Vec<String>, game: ESupportedGame) -> Self {
        Self {
            ext,
            game,
            rules: vec![],
        }
    }

    pub fn new_cyberpunk_parser() -> Self {
        Parser::new(vec![".archive".into()], ESupportedGame::Cyberpunk)
    }

    pub fn new_tes3_parser() -> Self {
        Parser::new(
            vec![".esp".into(), ".esm".into()],
            ESupportedGame::Morrowind,
        )
    }

    pub fn new_openmw_parser() -> Self {
        Parser::new(
            vec![".esp".into(), ".esm".into(), ".omwaddon".into()],
            ESupportedGame::OpenMorrowind,
        )
    }

    /// Parse rules for a specific game, expects the path to be the rules directory
    ///
    /// # Errors
    ///
    /// This function will return an error if file io or parsing fails
    pub fn parse<P>(&mut self, path: P) -> Result<Vec<Rule>>
    where
        P: AsRef<Path>,
    {
        self.rules.clear();

        match self.game {
            ESupportedGame::Morrowind => {
                {
                    let path = path.as_ref().join("mlox_base.txt");
                    if path.exists() {
                        if let Ok(base) = self.parse_rules_from_path(&path) {
                            info!("Parsed file {} with {} rules", path.display(), base.len());
                            self.rules.extend(base);
                        }
                    } else {
                        warn!("Could not find rules file {}", path.display());
                    }
                }

                {
                    let path = path.as_ref().join("mlox_user.txt");
                    if path.exists() {
                        if let Ok(base) = self.parse_rules_from_path(&path) {
                            info!("Parsed file {} with {} rules", path.display(), base.len());
                            self.rules.extend(base);
                        }
                    } else {
                        warn!("Could not find rules file {}", path.display());
                    }
                }

                {
                    let path = path.as_ref().join("mlox_my_rules.txt");
                    if path.exists() {
                        if let Ok(base) = self.parse_rules_from_path(&path) {
                            info!("Parsed file {} with {} rules", path.display(), base.len());
                            self.rules.extend(base);
                        }
                    } else {
                        warn!("Could not find rules file {}", path.display());
                    }
                }
            }
            ESupportedGame::OpenMorrowind => todo!(),
            ESupportedGame::Cyberpunk => todo!(),
        }

        // TODO dedup?
        info!("Parser initialized with {} rules", self.rules.len());
        Ok(self.rules.clone()) // TODO we clone here?
    }

    /// Parse rules from a rules file
    ///
    /// # Errors
    ///
    /// This function will return an error if file io or parsing fails
    pub fn parse_rules_from_path<P>(&self, path: P) -> Result<Vec<Rule>>
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
    pub fn parse_rules_from_reader<R>(&self, reader: R) -> Result<Vec<Rule>>
    where
        R: Read + BufRead + Seek,
    {
        // pre-parse into rule blocks
        let mut chunks: Vec<ChunkWrapper> = vec![];
        let mut chunk: Option<ChunkWrapper> = None;
        for (idx, line) in reader.lines().flatten().enumerate() {
            // ignore comments
            if line.trim_start().starts_with(';') {
                continue;
            }

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
        // TODO parallelize
        let mut rules: Vec<Rule> = vec![];
        for (idx, chunk) in chunks.into_iter().enumerate() {
            let info = &chunk.info;

            let cursor = Cursor::new(&chunk.data);
            let parsed = match self.parse_chunk(cursor) {
                Ok(it) => it,
                Err(err) => {
                    // log error and skip chunk
                    error!(
                        "Error '{}' at chunk #{}, starting at line: {}",
                        err, idx, info
                    );
                    let string = String::from_utf8(chunk.data).expect("not valid utf8");
                    debug!("{}", string);
                    continue;
                }
            };

            rules.extend(parsed);
        }

        Ok(rules)
    }

    /// Parses on rule section. Note: Order rules are returned as vec
    ///
    /// # Errors
    ///
    /// This function will return an error if parsing fails
    fn parse_chunk<R>(&self, mut reader: R) -> Result<Vec<Rule>>
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
                    let lowercase_line = line.to_lowercase();
                    if lowercase_line.strip_prefix("order").is_some() {
                        // Order lines don't have in-line options
                        rule = Order::default().into();
                    } else if let Some(rest) = lowercase_line.strip_prefix("note") {
                        let mut x = Note::default();
                        x.set_comment(rest.trim().to_owned());
                        rule = x.into();
                    } else if let Some(rest) = lowercase_line.strip_prefix("conflict") {
                        let mut x = Conflict::default();
                        x.set_comment(rest.trim().to_owned());
                        rule = x.into();
                    } else if let Some(rest) = lowercase_line.strip_prefix("requires") {
                        let mut x = Requires::default();
                        x.set_comment(rest.trim().to_owned());
                        rule = x.into();
                    } else {
                        // unknown rule, skip
                        return Err(Error::new(ErrorKind::Other, "Parsing error: unknown rule"));
                    }

                    // parse buffer
                    // some ad-hoc fixes because we have inline-rules
                    let mut lin = String::new();
                    reader.read_line(&mut lin)?;
                    lin = lin.trim_start().to_owned();

                    // ignore for order rules
                    if !lin.is_empty() {
                        // if the line is not empty we have an inline expression and we need to trim and read back to buffer
                        reader.seek(SeekFrom::Current(-(lin.len() as i64)))?;
                    }

                    // but we ignore comments
                    let mut body = vec![];
                    if let Some(_idx) = lin.find(';') {
                        // find idx
                        reader.read_until(b';', &mut body)?;
                        body.remove(body.len() - 1);
                    } else {
                        reader.read_to_end(&mut body)?;
                    }

                    let body_cursor = Cursor::new(body);

                    // now parse rule body
                    match rule {
                        // Order rules don't have comments and no expressions so we can just parse them individually
                        Rule::Order(_) => self.parse_order_rule(body_cursor),
                        mut x => {
                            Rule::parse(&mut x, body_cursor, self)?;
                            Ok(vec![x])
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

    /// .Parse an order rule, it can have up to N items
    ///
    /// # Errors
    ///
    /// This function will return an error if Order rule is missformed
    fn parse_order_rule<R>(&self, reader: R) -> Result<Vec<Rule>>
    where
        R: Read + BufRead,
    {
        let mut order: Vec<String> = vec![];

        // parse each line
        for line in reader.lines().flatten().map(|l| l.trim().to_owned()) {
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
        let mut rules: Vec<Rule> = vec![];
        match order.len().cmp(&2) {
            Ordering::Less => {
                // Rule with only one element is an error
                return Err(Error::new(
                    ErrorKind::Other,
                    "Logic error: order rule with less than two elements",
                ));
            }
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
                } else {
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

        let mut expressions: Vec<Expression> = vec![];
        for str in buffers {
            let trimmed = str.trim();
            if trimmed.is_empty() {
                continue;
            }
            let expr = self.parse_expression(trimmed)?;
            expressions.push(expr);
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
            if let Some(rest) = reader.strip_prefix("[ANY ") {
                let expressions = self.parse_expressions(rest[..rest.len() - 1].as_bytes())?;
                let expr = ANY::new(expressions);
                Ok(expr.into())
            } else if let Some(rest) = reader.strip_prefix("[ALL") {
                let expressions = self.parse_expressions(rest[..rest.len() - 1].as_bytes())?;
                let expr = ALL::new(expressions);
                Ok(expr.into())
            } else if let Some(rest) = reader.strip_prefix("[NOT") {
                let expressions = self.parse_expressions(rest[..rest.len() - 1].as_bytes())?;
                if let Some(first) = expressions.into_iter().last() {
                    let expr = NOT::new(first);
                    Ok(expr.into())
                } else {
                    Err(Error::new(
                        ErrorKind::Other,
                        "Parsing error: unknown expression",
                    ))
                }
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
}

/// .Reads the first comment lines of a rule chunk and returns the rest as byte buffer
///
/// # Errors
///
/// This function will return an error if stream reading or seeking fails
pub fn read_comment<R: Read + BufRead + Seek>(reader: &mut R) -> Result<Option<String>> {
    // a line starting with a whitespace may be a comment
    if reader.read_u8()? as char != ' ' {
        reader.seek(SeekFrom::Current(-1))?;
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
