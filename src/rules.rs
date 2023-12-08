////////////////////////////////////////////////////////////////////////
/// RULES
////////////////////////////////////////////////////////////////////////
use crate::expressions::Expression;

#[derive(Default)]
pub struct Rules {
    pub order: Vec<(String, String)>,
}

// todo replace with pattern matching
pub enum RuleKind {
    Order,
    Note,
    Conflict,
    Require,
}

/// A rule as specified in the rules document
pub trait Rule {
    /// todo replace with pattern matching
    fn get_kind(&self) -> RuleKind;
    // every rule may have a comment describing why it failed
    fn get_comment(&self) -> &str;
    // every rule may be evaluated
    fn eval(&self, items: &[String]) -> bool;
}

/// The Note Rule <Note for A>
/// Notes simply check the expression and notify the user if eval is true
pub struct Note {
    pub comment: String,
    pub expression: Box<dyn Expression>,
}
impl Rule for Note {
    fn get_kind(&self) -> RuleKind {
        RuleKind::Note
    }
    fn get_comment(&self) -> &str {
        &self.comment
    }
    /// Notes evaluate as true if the expression evaluates as true
    fn eval(&self, items: &[String]) -> bool {
        self.expression.eval(items)
    }
}

/// The Conflict Rule <A conflicts with B>
/// Conflicts evaluate as true if both expressions evaluate as true
pub struct Conflict {
    pub comment: String,
    // todo: make first atomic?
    pub expression_a: Box<dyn Expression>,
    pub expression_b: Box<dyn Expression>,
}
impl Rule for Conflict {
    fn get_kind(&self) -> RuleKind {
        RuleKind::Conflict
    }
    fn get_comment(&self) -> &str {
        &self.comment
    }
    /// Conflicts evaluate as true if both expressions evaluate as true
    fn eval(&self, items: &[String]) -> bool {
        self.expression_a.eval(items) && self.expression_b.eval(items)
    }
}

/// The Require Rule <A requires B>
/// Requires evaluates as true if A is true and B is not true
pub struct Require {
    pub comment: String,
    // todo: make first atomic?
    pub expression_a: Box<dyn Expression>,
    pub expression_b: Box<dyn Expression>,
}
impl Rule for Require {
    fn get_kind(&self) -> RuleKind {
        RuleKind::Require
    }
    fn get_comment(&self) -> &str {
        &self.comment
    }
    /// Requires evaluates as true if A is true and B is not true
    fn eval(&self, items: &[String]) -> bool {
        self.expression_a.eval(items) && !self.expression_b.eval(items)
    }
}
