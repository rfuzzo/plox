////////////////////////////////////////////////////////////////////////
// RULES
////////////////////////////////////////////////////////////////////////
use std::{
    fmt::Display,
    io::{BufRead, Error, ErrorKind, Read, Result, Seek},
};

use log::warn;
use serde::{Deserialize, Serialize};

use crate::{expressions::*, parser, PluginData};

///////////////////////////////////////////////////
// ENUMS

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ERule {
    EOrderRule(EOrderRule),
    EWarningRule(EWarningRule),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EOrderRule {
    Order(Order),
    NearStart(NearStart),
    NearEnd(NearEnd),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    fn get_plugins(&self) -> Vec<String>;

    fn set_comment(&mut self, comment: String);
    /// every rule may be evaluated
    fn eval(&mut self, items: &[PluginData]) -> bool;
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

    fn get_plugins(&self) -> Vec<String> {
        match self {
            EWarningRule::Note(x) => x.get_plugins(),
            EWarningRule::Conflict(x) => x.get_plugins(),
            EWarningRule::Requires(x) => x.get_plugins(),
            EWarningRule::Patch(x) => x.get_plugins(),
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

    fn eval(&mut self, items: &[PluginData]) -> bool {
        match self {
            EWarningRule::Note(o) => o.eval(items),
            EWarningRule::Conflict(o) => o.eval(items),
            EWarningRule::Requires(o) => o.eval(items),
            EWarningRule::Patch(o) => o.eval(items),
        }
    }
}

pub trait TParser<T> {
    fn parse<R: Read + BufRead + Seek>(
        rule: &mut T,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()>;
}

impl TParser<ERule> for ERule {
    fn parse<R: Read + BufRead + Seek>(
        rule: &mut ERule,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        match rule {
            ERule::EOrderRule(rule) => EOrderRule::parse(rule, reader, parser),
            ERule::EWarningRule(rule) => EWarningRule::parse(rule, reader, parser),
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
            EWarningRule::Note(rule) => Note::parse(rule, reader, parser),
            EWarningRule::Conflict(rule) => Conflict::parse(rule, reader, parser),
            EWarningRule::Requires(rule) => Requires::parse(rule, reader, parser),
            EWarningRule::Patch(rule) => Patch::parse(rule, reader, parser),
        }
    }
}

impl TParser<EOrderRule> for EOrderRule {
    fn parse<R: Read + BufRead + Seek>(
        rule: &mut EOrderRule,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        match rule {
            EOrderRule::Order(rule) => Order::parse(rule, reader, parser),
            EOrderRule::NearStart(rule) => NearStart::parse(rule, reader, parser),
            EOrderRule::NearEnd(rule) => NearEnd::parse(rule, reader, parser),
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
        ERule::EWarningRule(val)
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
        ERule::EWarningRule(val.into())
    }
}
impl From<Conflict> for ERule {
    fn from(val: Conflict) -> Self {
        ERule::EWarningRule(val.into())
    }
}
impl From<Requires> for ERule {
    fn from(val: Requires) -> Self {
        ERule::EWarningRule(val.into())
    }
}
impl From<Patch> for ERule {
    fn from(val: Patch) -> Self {
        ERule::EWarningRule(val.into())
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
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Order {
    pub names: Vec<String>,
}
impl Order {
    pub fn new(names: Vec<String>) -> Self {
        Self { names }
    }

    pub fn from(name_a: &str, name_b: &str) -> Self {
        Self {
            names: [name_a.to_owned(), name_b.to_owned()].to_vec(),
        }
    }
}
impl TParser<Order> for Order {
    fn parse<R: Read + BufRead + Seek>(
        this: &mut Order,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        // parse each line
        let mut names: Vec<String> = vec![];
        for line in reader
            .lines()
            .map_while(Result::ok)
            .map(|l| l.trim().to_owned())
        {
            // HANDLE RULE PARSE
            // each line gets tokenized
            for token in parser.tokenize(line) {
                if !token.ends_with(']') && !parser.ends_with_vec(&token) {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Parsing error: tokenize failed",
                    ));
                }
                names.push(token);
            }
        }

        this.names = names;

        if this.names.len() < 2 {
            warn!("Malformed Order rule: less than 2 expressions");
            return Err(Error::new(
                ErrorKind::Other,
                "Malformed Order rule: less than 2 expressions",
            ));
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////
// NEARSTART

/// The [NearStart] rule specifies that one or more plugins should appear as near as possible to the Start of the load order.
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct NearStart {
    pub names: Vec<String>,
}
impl NearStart {
    pub fn new(names: Vec<String>) -> Self {
        Self { names }
    }
}
impl TParser<NearStart> for NearStart {
    fn parse<R: Read + BufRead + Seek>(
        this: &mut NearStart,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        // parse each line
        let mut names: Vec<String> = vec![];
        for line in reader
            .lines()
            .map_while(Result::ok)
            .map(|l| l.trim().to_owned())
        {
            // HANDLE RULE PARSE
            // each line gets tokenized
            for token in parser.tokenize(line) {
                if !token.ends_with(']') && !parser.ends_with_vec(&token) {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Parsing error: tokenize failed",
                    ));
                }
                names.push(token);
            }
        }

        this.names = names;

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////
// NEAREND

/// The [NearEnd] rule specifies that one or more plugins should appear as near as possible to the End of the load order.
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct NearEnd {
    pub names: Vec<String>,
}
impl NearEnd {
    pub fn new(names: Vec<String>) -> Self {
        Self { names }
    }
}
impl TParser<NearEnd> for NearEnd {
    fn parse<R: Read + BufRead + Seek>(
        this: &mut NearEnd,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        // parse each line
        let mut names: Vec<String> = vec![];
        for line in reader
            .lines()
            .map_while(Result::ok)
            .map(|l| l.trim().to_owned())
        {
            // HANDLE RULE PARSE
            // each line gets tokenized
            for token in parser.tokenize(line) {
                if !token.ends_with(']') && !parser.ends_with_vec(&token) {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Parsing error: tokenize failed",
                    ));
                }
                names.push(token);
            }
        }

        this.names = names;

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////
// IMPLEMENTATIONS WARNINGS
////////////////////////////////////////////////////////////////////////

impl Display for EWarningRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EWarningRule::Note(x) => x.fmt(f),
            EWarningRule::Conflict(x) => x.fmt(f),
            EWarningRule::Requires(x) => x.fmt(f),
            EWarningRule::Patch(x) => x.fmt(f),
        }
    }
}

////////////////////////////////////////////////////////////////////////
// NOTE

/// The [Note] Rule <Note for A>
/// The [Note] rule prints the given message when any of the following expressions is true.
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Note {
    pub comment: String,
    pub expressions: Vec<Expression>,

    pub plugins: Vec<String>,
}

impl Note {
    pub fn new(comment: String, expressions: &[Expression]) -> Self {
        Self {
            comment,
            expressions: expressions.to_vec(),
            plugins: vec![],
        }
    }
}
impl TWarningRule for Note {
    fn get_comment(&self) -> &str {
        self.comment.as_str()
    }
    fn get_plugins(&self) -> Vec<String> {
        self.plugins.clone()
    }

    fn set_comment(&mut self, comment: String) {
        self.comment = comment;
    }
    /// Notes evaluate as true if any of the containing expressions evaluates as true
    fn eval(&mut self, items: &[PluginData]) -> bool {
        let mut result = false;
        for expr in &self.expressions {
            if let Some(plugins) = expr.eval(items) {
                result = true;
                // track plugins
                self.plugins.extend(plugins);
            }
        }
        result
    }
}
impl TParser<Note> for Note {
    fn parse<R: Read + BufRead + Seek>(
        this: &mut Note,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        // if let Ok(Some(comment)) = read_comment(&mut reader) {
        //     this.set_comment(comment);
        // }

        // add all parsed expressions
        this.expressions = parser.parse_expressions(reader)?;

        if this.expressions.is_empty() {
            warn!("Malformed Note rule: no expressions parsed");
            return Err(Error::new(
                ErrorKind::Other,
                "Malformed Note rule: no expressions parsed",
            ));
        }

        Ok(())
    }
}

impl Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut block = format!("[NOTE]\n\t{}\n", self.comment);
        for e in &self.expressions {
            block += format!("{}\n", e).as_str();
        }
        write!(f, "{}", block)
    }
}

////////////////////////////////////////////////////////////////////////
// CONFLICT

/// The [Conflict] Rule <A conflicts with B>
/// [Conflict] evaluate as true if both expressions evaluate as true
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Conflict {
    pub comment: String,
    pub expressions: Vec<Expression>,

    pub plugins: Vec<String>,
}
impl Conflict {
    pub fn new(comment: String, expressions: &[Expression]) -> Self {
        Self {
            comment,
            expressions: expressions.to_vec(),
            plugins: vec![],
        }
    }
}
impl TWarningRule for Conflict {
    fn get_comment(&self) -> &str {
        self.comment.as_str()
    }
    fn get_plugins(&self) -> Vec<String> {
        self.plugins.clone()
    }
    fn set_comment(&mut self, comment: String) {
        self.comment = comment;
    }
    /// Conflicts evaluate as true if both expressions evaluate as true
    fn eval(&mut self, items: &[PluginData]) -> bool {
        let mut i = 0;
        for e in &self.expressions {
            if let Some(plugins) = e.eval(items) {
                self.plugins.extend(plugins);
                i += 1;
            }
        }

        i > 1
    }
}
impl TParser<Conflict> for Conflict {
    fn parse<R: Read + BufRead + Seek>(
        this: &mut Conflict,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        // if let Ok(Some(comment)) = read_comment(&mut reader) {
        //     this.set_comment(comment);
        // }

        // add all parsed expressions
        this.expressions = parser.parse_expressions(reader)?;

        if this.expressions.is_empty() {
            warn!("Malformed Conflict rule: no expressions parsed");
            return Err(Error::new(
                ErrorKind::Other,
                "Malformed Conflict rule: no expressions parsed",
            ));
        }

        Ok(())
    }
}

impl Display for Conflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut block = format!("[CONFLICT]\n\t{}\n", self.comment);
        for e in &self.expressions {
            block += format!("{}\n", e).as_str();
        }
        write!(f, "{}", block)
    }
}

////////////////////////////////////////////////////////////////////////
// REQUIRES

/// The [Requires] Rule <A requires B>
/// [Requires] evaluates as true if A is true and B is not true
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Requires {
    pub comment: String,
    pub expression_a: Option<Expression>,
    pub expression_b: Option<Expression>,

    pub plugins: Vec<String>,
}
impl Requires {
    pub fn new(comment: String, expression_a: Expression, expression_b: Expression) -> Self {
        Self {
            comment,
            expression_a: Some(expression_a),
            expression_b: Some(expression_b),
            plugins: vec![],
        }
    }
}
impl TWarningRule for Requires {
    fn get_comment(&self) -> &str {
        self.comment.as_str()
    }
    fn get_plugins(&self) -> Vec<String> {
        self.plugins.clone()
    }

    fn set_comment(&mut self, comment: String) {
        self.comment = comment;
    }
    /// Requires evaluates as true if A is true and B is not true
    fn eval(&mut self, items: &[PluginData]) -> bool {
        let mut result = false;
        if let Some(expr_a) = &self.expression_a {
            if let Some(expr_b) = &self.expression_b {
                if let Some(plugins) = expr_a.eval(items) {
                    if expr_b.eval(items).is_none() {
                        result = true;
                        self.plugins.extend(plugins);
                    }
                }
            }
        }
        result
    }
}

impl TParser<Requires> for Requires {
    fn parse<R: Read + BufRead + Seek>(
        this: &mut Requires,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        // if let Ok(Some(comment)) = read_comment(&mut reader) {
        //     this.set_comment(comment);
        // }

        // add all parsed expressions
        let expressions = parser.parse_expressions(reader)?;
        if expressions.len() != 2 {
            warn!("Malformed Requires rule: more than 2 expressions");
            return Err(Error::new(
                ErrorKind::Other,
                "Malformed Requires rule: more than 2 expressions",
            ));
        }

        this.expression_a = Some(expressions[0].clone());
        this.expression_b = Some(expressions[1].clone());

        Ok(())
    }
}

impl Display for Requires {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut block = format!("[REQUIRES]\n\t{}\n", self.comment);
        if let Some(a) = &self.expression_a {
            block += format!("{}\n", a).as_str();
        }
        if let Some(b) = &self.expression_b {
            block += format!("{}\n", b).as_str();
        }
        write!(f, "{}", block)
    }
}

////////////////////////////////////////////////////////////////////////
// PATCH

/// The [Patch] rule specifies a mutual dependency
/// we wouldn't want the patch without the original it is supposed to patch
/// We wouldn't want the original to go unpatched.
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Patch {
    pub comment: String,
    pub expression_a: Option<Expression>,
    pub expression_b: Option<Expression>,

    pub plugins: Vec<String>,
}
impl Patch {
    pub fn new(comment: String, expression_a: Expression, expression_b: Expression) -> Self {
        Self {
            comment,
            expression_a: Some(expression_a),
            expression_b: Some(expression_b),
            plugins: vec![],
        }
    }
}
impl TWarningRule for Patch {
    fn get_comment(&self) -> &str {
        self.comment.as_str()
    }
    fn get_plugins(&self) -> Vec<String> {
        self.plugins.clone()
    }

    fn set_comment(&mut self, comment: String) {
        self.comment = comment;
    }
    /// Patch evaluates as true if A is true and B is not true or if B is true and A is not true
    fn eval(&mut self, items: &[PluginData]) -> bool {
        let mut result = false;
        if let Some(expr_a) = &self.expression_a {
            if let Some(expr_b) = &self.expression_b {
                if let Some(plugins_a) = expr_a.eval(items) {
                    if expr_b.eval(items).is_none() {
                        result = true;
                        self.plugins.extend(plugins_a);
                    }
                }

                if let Some(plugins_b) = expr_b.eval(items) {
                    if expr_a.eval(items).is_none() {
                        result = true;
                        self.plugins.extend(plugins_b);
                    }
                }
            }
        }
        result
    }
}

impl TParser<Patch> for Patch {
    fn parse<R: Read + BufRead + Seek>(
        this: &mut Patch,
        reader: R,
        parser: &parser::Parser,
    ) -> Result<()> {
        // if let Ok(Some(comment)) = read_comment(&mut reader) {
        //     this.set_comment(comment);
        // }

        // add all parsed expressions
        let expressions = parser.parse_expressions(reader)?;
        if expressions.len() != 2 {
            warn!("Malformed Patch rule: not exactly 2 expressions");
            return Err(Error::new(
                ErrorKind::Other,
                "Malformed Patch rule: not exactly 2 expressions",
            ));
        }

        this.expression_a = Some(expressions[0].clone());
        this.expression_b = Some(expressions[1].clone());

        Ok(())
    }
}

impl Display for Patch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut block = format!("[PATCH]\n\t{}\n", self.comment);
        if let Some(a) = &self.expression_a {
            block += format!("{}\n", a).as_str();
        }
        if let Some(b) = &self.expression_b {
            block += format!("{}\n", b).as_str();
        }
        write!(f, "{}", block)
    }
}
