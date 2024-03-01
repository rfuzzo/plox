////////////////////////////////////////////////////////////////////////
// RULES
////////////////////////////////////////////////////////////////////////
use std::io::{BufRead, Error, ErrorKind, Read, Result, Seek};

use crate::{
    expressions::*,
    parser::{self, read_comment},
};

/// A rule as specified in the rules document
pub trait TRule {
    /// every rule may have a comment describing why it failed
    fn get_comment(&self) -> &str;
    fn set_comment(&mut self, comment: String);
    /// every rule may be evaluated
    fn eval(&self, items: &[String]) -> bool;
}

pub trait TParser<T: TRule> {
    fn parse<R: Read + BufRead + Seek>(
        rule: &mut T,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum Rule {
    Order(Order),
    Note(Note),
    Conflict(Conflict),
    Requires(Requires),
}

impl TRule for Rule {
    fn get_comment(&self) -> &str {
        match self {
            Rule::Order(x) => x.get_comment(),
            Rule::Note(x) => x.get_comment(),
            Rule::Conflict(x) => x.get_comment(),
            Rule::Requires(x) => x.get_comment(),
        }
    }

    fn set_comment(&mut self, comment: String) {
        match self {
            Rule::Order(x) => x.set_comment(comment),
            Rule::Note(x) => x.set_comment(comment),
            Rule::Conflict(x) => x.set_comment(comment),
            Rule::Requires(x) => x.set_comment(comment),
        }
    }

    fn eval(&self, items: &[String]) -> bool {
        match self {
            Rule::Order(o) => o.eval(items),
            Rule::Note(o) => o.eval(items),
            Rule::Conflict(o) => o.eval(items),
            Rule::Requires(o) => o.eval(items),
        }
    }
}

impl TParser<Rule> for Rule {
    fn parse<R: Read + BufRead + Seek>(
        rule: &mut Rule,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        match rule {
            Rule::Order(_) => {
                // order rules are not parsed like this
                Err(Error::new(
                    ErrorKind::Other,
                    "Parsing error: Trying to Parse Order rule",
                ))
            }
            Rule::Note(x) => Note::parse(x, reader, parser),
            Rule::Conflict(x) => Conflict::parse(x, reader, parser),
            Rule::Requires(x) => Requires::parse(x, reader, parser),
        }
    }
}

// conversions
impl From<Order> for Rule {
    fn from(val: Order) -> Self {
        Rule::Order(val)
    }
}
impl From<Note> for Rule {
    fn from(val: Note) -> Self {
        Rule::Note(val)
    }
}
impl From<Conflict> for Rule {
    fn from(val: Conflict) -> Self {
        Rule::Conflict(val)
    }
}
impl From<Requires> for Rule {
    fn from(val: Requires) -> Self {
        Rule::Requires(val)
    }
}

////////////////////////////////////////////////////////////////////////
// IMPLEMENTATIONS
////////////////////////////////////////////////////////////////////////

////////////////////////////////////////////////////////////////////////
// ORDER

/// The Note Rule <Note for A>
/// Notes simply check the expression and notify the user if eval is true
#[derive(Default, Clone, Debug)]
pub struct Order {
    pub name_a: String,
    pub name_b: String,
}

impl Order {
    pub fn new(name_a: String, name_b: String) -> Self {
        Self { name_a, name_b }
    }
}
impl TRule for Order {
    fn get_comment(&self) -> &str {
        ""
    }
    fn set_comment(&mut self, _comment: String) {}

    fn eval(&self, _items: &[String]) -> bool {
        false
    }
}

////////////////////////////////////////////////////////////////////////
// NOTE

/// The Note Rule <Note for A>
/// The [Note] rule prints the given message when any of the following expressions is true.
#[derive(Default, Clone, Debug)]
pub struct Note {
    pub comment: String,
    pub expressions: Vec<Expression>,
}

impl Note {
    pub fn new(comment: String, expressions: &[Expression]) -> Self {
        Self {
            comment,
            expressions: expressions.to_vec(),
        }
    }
}
impl TRule for Note {
    fn get_comment(&self) -> &str {
        self.comment.as_str()
    }
    fn set_comment(&mut self, comment: String) {
        self.comment = comment;
    }
    /// Notes evaluate as true if any of the containing expressions evaluates as true
    fn eval(&self, items: &[String]) -> bool {
        for expr in &self.expressions {
            if expr.eval(items) {
                return true;
            }
        }
        false
    }
}
impl TParser<Note> for Note {
    fn parse<R: Read + BufRead + Seek>(
        this: &mut Note,
        mut reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        if let Ok(Some(comment)) = read_comment(&mut reader) {
            this.set_comment(comment);
        }

        // add all parsed expressions
        this.expressions = parser.parse_expressions(reader)?;

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////
// CONFLICT

/// The Conflict Rule <A conflicts with B>
/// Conflicts evaluate as true if both expressions evaluate as true
#[derive(Default, Clone, Debug)]
pub struct Conflict {
    pub comment: String,
    pub expression_a: Option<Expression>,
    pub expression_b: Option<Expression>,
}
impl Conflict {
    pub fn new(comment: String, expression_a: Expression, expression_b: Expression) -> Self {
        Self {
            comment,
            expression_a: Some(expression_a),
            expression_b: Some(expression_b),
        }
    }
}
impl TRule for Conflict {
    fn get_comment(&self) -> &str {
        self.comment.as_str()
    }
    fn set_comment(&mut self, comment: String) {
        self.comment = comment;
    }
    /// Conflicts evaluate as true if both expressions evaluate as true
    fn eval(&self, items: &[String]) -> bool {
        if let Some(expr_a) = &self.expression_a {
            if let Some(expr_b) = &self.expression_b {
                return expr_a.eval(items) && expr_b.eval(items);
            }
        }
        false
    }
}
impl TParser<Conflict> for Conflict {
    fn parse<R: Read + BufRead + Seek>(
        this: &mut Conflict,
        mut reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        if let Ok(Some(comment)) = read_comment(&mut reader) {
            this.set_comment(comment);
        }

        // add all parsed expressions
        let expressions = parser.parse_expressions(reader)?;
        for (i, e) in expressions.into_iter().enumerate() {
            match i {
                0 => {
                    this.expression_a = Some(e);
                }
                1 => {
                    this.expression_b = Some(e);
                }
                _ => {}
            }
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////
// REQUIRES

/// The Requires Rule <A requires B>
/// Requires evaluates as true if A is true and B is not true
#[derive(Default, Clone, Debug)]
pub struct Requires {
    pub comment: String,
    pub expression_a: Option<Expression>,
    pub expression_b: Option<Expression>,
}
impl Requires {
    pub fn new(comment: String, expression_a: Expression, expression_b: Expression) -> Self {
        Self {
            comment,
            expression_a: Some(expression_a),
            expression_b: Some(expression_b),
        }
    }
}
impl TRule for Requires {
    fn get_comment(&self) -> &str {
        self.comment.as_str()
    }
    fn set_comment(&mut self, comment: String) {
        self.comment = comment;
    }
    /// Requires evaluates as true if A is true and B is not true
    fn eval(&self, items: &[String]) -> bool {
        if let Some(expr_a) = &self.expression_a {
            if let Some(expr_b) = &self.expression_b {
                return expr_a.eval(items) && !expr_b.eval(items);
            }
        }
        false
    }
}

impl TParser<Requires> for Requires {
    fn parse<R: Read + BufRead + Seek>(
        this: &mut Requires,
        mut reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        if let Ok(Some(comment)) = read_comment(&mut reader) {
            this.set_comment(comment);
        }

        // add all parsed expressions
        let expressions = parser.parse_expressions(reader)?;
        for (i, e) in expressions.into_iter().enumerate() {
            match i {
                0 => {
                    this.expression_a = Some(e);
                }
                1 => {
                    this.expression_b = Some(e);
                }
                _ => {}
            }
        }

        Ok(())
    }
}
