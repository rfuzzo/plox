////////////////////////////////////////////////////////////////////////
// RULES
////////////////////////////////////////////////////////////////////////
use std::io::{BufRead, Error, ErrorKind, Read, Result, Seek};

use log::warn;

use crate::{
    expressions::*,
    parser::{self, read_comment},
};

///////////////////////////////////////////////////
// ENUMS

#[derive(Debug, Clone)]
pub enum ERule {
    EOrderRule(EOrderRule),
    Rule(EWarningRule),
}

#[derive(Debug, Clone)]
pub enum EOrderRule {
    Order(Order),
    NearStart(NearStart),
    NearEnd(NearEnd),
}

#[derive(Debug, Clone)]
pub enum EWarningRule {
    Note(Note),
    Conflict(Conflict),
    Requires(Requires),
    Patch(Patch),
}

///////////////////////////////////////////////////
// TRAITS

/// A rule as specified in the rules document
pub trait TWarningRule {
    /// every rule may have a comment describing why it failed
    fn get_comment(&self) -> &str;
    fn set_comment(&mut self, comment: String);
    /// every rule may be evaluated
    fn eval(&self, items: &[String]) -> bool;
}

pub trait TParser<T: TWarningRule> {
    fn parse<R: Read + BufRead + Seek>(
        rule: &mut T,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()>;
}

impl TWarningRule for EWarningRule {
    fn get_comment(&self) -> &str {
        match self {
            EWarningRule::Note(x) => x.get_comment(),
            EWarningRule::Conflict(x) => x.get_comment(),
            EWarningRule::Requires(x) => x.get_comment(),
            EWarningRule::Patch(x) => x.get_comment(),
        }
    }

    fn set_comment(&mut self, comment: String) {
        match self {
            EWarningRule::Note(x) => x.set_comment(comment),
            EWarningRule::Conflict(x) => x.set_comment(comment),
            EWarningRule::Requires(x) => x.set_comment(comment),
            EWarningRule::Patch(x) => x.set_comment(comment),
        }
    }

    fn eval(&self, items: &[String]) -> bool {
        match self {
            EWarningRule::Note(o) => o.eval(items),
            EWarningRule::Conflict(o) => o.eval(items),
            EWarningRule::Requires(o) => o.eval(items),
            EWarningRule::Patch(o) => o.eval(items),
        }
    }
}

impl TParser<EWarningRule> for EWarningRule {
    fn parse<R: Read + BufRead + Seek>(
        rule: &mut EWarningRule,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        match rule {
            EWarningRule::Note(x) => Note::parse(x, reader, parser),
            EWarningRule::Conflict(x) => Conflict::parse(x, reader, parser),
            EWarningRule::Requires(x) => Requires::parse(x, reader, parser),
            EWarningRule::Patch(x) => Patch::parse(x, reader, parser),
        }
    }
}

// conversions
// top level
impl From<EOrderRule> for ERule {
    fn from(val: EOrderRule) -> Self {
        ERule::EOrderRule(val)
    }
}
impl From<EWarningRule> for ERule {
    fn from(val: EWarningRule) -> Self {
        ERule::Rule(val)
    }
}

// Order
impl From<Order> for ERule {
    fn from(val: Order) -> Self {
        ERule::EOrderRule(val.into())
    }
}
impl From<NearStart> for ERule {
    fn from(val: NearStart) -> Self {
        ERule::EOrderRule(val.into())
    }
}
impl From<NearEnd> for ERule {
    fn from(val: NearEnd) -> Self {
        ERule::EOrderRule(val.into())
    }
}

impl From<Order> for EOrderRule {
    fn from(val: Order) -> Self {
        EOrderRule::Order(val)
    }
}
impl From<NearStart> for EOrderRule {
    fn from(val: NearStart) -> Self {
        EOrderRule::NearStart(val)
    }
}
impl From<NearEnd> for EOrderRule {
    fn from(val: NearEnd) -> Self {
        EOrderRule::NearEnd(val)
    }
}

// Warnings
impl From<Note> for ERule {
    fn from(val: Note) -> Self {
        ERule::Rule(val.into())
    }
}
impl From<Conflict> for ERule {
    fn from(val: Conflict) -> Self {
        ERule::Rule(val.into())
    }
}
impl From<Requires> for ERule {
    fn from(val: Requires) -> Self {
        ERule::Rule(val.into())
    }
}
impl From<Patch> for ERule {
    fn from(val: Patch) -> Self {
        ERule::Rule(val.into())
    }
}

impl From<Note> for EWarningRule {
    fn from(val: Note) -> Self {
        EWarningRule::Note(val)
    }
}
impl From<Conflict> for EWarningRule {
    fn from(val: Conflict) -> Self {
        EWarningRule::Conflict(val)
    }
}
impl From<Requires> for EWarningRule {
    fn from(val: Requires) -> Self {
        EWarningRule::Requires(val)
    }
}
impl From<Patch> for EWarningRule {
    fn from(val: Patch) -> Self {
        EWarningRule::Patch(val)
    }
}

////////////////////////////////////////////////////////////////////////
// IMPLEMENTATIONS ORDER
////////////////////////////////////////////////////////////////////////

////////////////////////////////////////////////////////////////////////
// ORDER

/// The [Order] rule specifies the order of plugins.
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

////////////////////////////////////////////////////////////////////////
// NEARSTART

/// The [NearStart] rule specifies that one or more plugins should appear as near as possible to the Start of the load order.
#[derive(Default, Clone, Debug)]
pub struct NearStart {
    pub names: Vec<String>,
}

impl NearStart {
    pub fn new(names: Vec<String>) -> Self {
        Self { names }
    }
}

////////////////////////////////////////////////////////////////////////
// NEAREND

/// The [NearEnd] rule specifies that one or more plugins should appear as near as possible to the End of the load order.
#[derive(Default, Clone, Debug)]
pub struct NearEnd {
    pub names: Vec<String>,
}

impl NearEnd {
    pub fn new(names: Vec<String>) -> Self {
        Self { names }
    }
}

////////////////////////////////////////////////////////////////////////
// IMPLEMENTATIONS WARNINGS
////////////////////////////////////////////////////////////////////////

////////////////////////////////////////////////////////////////////////
// NOTE

/// The [Note] Rule <Note for A>
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
impl TWarningRule for Note {
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

/// The [Conflict] Rule <A conflicts with B>
/// [Conflict] evaluate as true if both expressions evaluate as true
#[derive(Default, Clone, Debug)]
pub struct Conflict {
    pub comment: String,
    pub expressions: Vec<Expression>,
}
impl Conflict {
    pub fn new(comment: String, expressions: &[Expression]) -> Self {
        Self {
            comment,
            expressions: expressions.to_vec(),
        }
    }
}
impl TWarningRule for Conflict {
    fn get_comment(&self) -> &str {
        self.comment.as_str()
    }
    fn set_comment(&mut self, comment: String) {
        self.comment = comment;
    }
    /// Conflicts evaluate as true if both expressions evaluate as true
    fn eval(&self, items: &[String]) -> bool {
        let mut i = 0;
        for e in &self.expressions {
            if e.eval(items) {
                i += 1;
            }
        }

        i > 1
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
        this.expressions = parser.parse_expressions(reader)?;
        if this.expressions.is_empty() {
            warn!("Malformed Conflict rule: less than 2 expressions");
            return Err(Error::new(
                ErrorKind::Other,
                "Malformed Conflict rule: less than 2 expressions",
            ));
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////
// REQUIRES

/// The [Requires] Rule <A requires B>
/// [Requires] evaluates as true if A is true and B is not true
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
impl TWarningRule for Requires {
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
        if expressions.len() != 2 {
            warn!("Malformed Requires rule: more than 2 expressions");
            return Err(Error::new(
                ErrorKind::Other,
                "Malformed Requires rule: more than 2 expressions",
            ));
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////
// PATCH

/// The [Patch] rule specifies a mutual dependency
/// we wouldn't want the patch without the original it is supposed to patch
/// We wouldn't want the original to go unpatched.
#[derive(Default, Clone, Debug)]
pub struct Patch {
    pub comment: String,
    pub expression_a: Option<Expression>,
    pub expression_b: Option<Expression>,
}
impl Patch {
    pub fn new(comment: String, expression_a: Expression, expression_b: Expression) -> Self {
        Self {
            comment,
            expression_a: Some(expression_a),
            expression_b: Some(expression_b),
        }
    }
}
impl TWarningRule for Patch {
    fn get_comment(&self) -> &str {
        self.comment.as_str()
    }
    fn set_comment(&mut self, comment: String) {
        self.comment = comment;
    }
    /// Patch evaluates as true if A is true and B is not true or if B is true and A is not true
    fn eval(&self, items: &[String]) -> bool {
        if let Some(expr_a) = &self.expression_a {
            if let Some(expr_b) = &self.expression_b {
                return (expr_a.eval(items) && !expr_b.eval(items))
                    || (expr_b.eval(items) && !expr_a.eval(items));
            }
        }
        false
    }
}

impl TParser<Patch> for Patch {
    fn parse<R: Read + BufRead + Seek>(
        this: &mut Patch,
        mut reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        if let Ok(Some(comment)) = read_comment(&mut reader) {
            this.set_comment(comment);
        }

        // add all parsed expressions
        let expressions = parser.parse_expressions(reader)?;
        if expressions.len() != 2 {
            warn!("Malformed Patch rule: more than 2 expressions");
            return Err(Error::new(
                ErrorKind::Other,
                "Malformed Patch rule: more than 2 expressions",
            ));
        }

        Ok(())
    }
}
