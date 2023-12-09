////////////////////////////////////////////////////////////////////////
/// RULES
////////////////////////////////////////////////////////////////////////
use crate::expressions::*;

pub enum RuleKind {
    Order(Order), // TODO refactor this into a rule?
    Note(Note),
    Conflict(Conflict),
    Requires(Requires),
}

/// A rule as specified in the rules document
pub trait Rule {
    // every rule may have a comment describing why it failed
    fn get_comment(&self) -> &str;
    // every rule may be evaluated
    fn eval(&self, items: &[String]) -> bool;
}

////////////////////////////////////////////////////////////////////////
/// IMPLEMENTATIONS
////////////////////////////////////////////////////////////////////////

/// The Note Rule <Note for A>
/// Notes simply check the expression and notify the user if eval is true
#[derive(Default, Clone)]
pub struct Order {
    pub name_a: String,
    pub name_b: String,
}

impl Order {
    pub fn new(name_a: String, name_b: String) -> Self {
        Self { name_a, name_b }
    }
}
impl Rule for Order {
    fn get_comment(&self) -> &str {
        ""
    }
    /// Notes evaluate as true if the expression evaluates as true
    fn eval(&self, _items: &[String]) -> bool {
        false
    }
}

/// The Note Rule <Note for A>
/// The [Note] rule prints the given message when any of the following expressions is true.
#[derive(Default, Clone)]
pub struct Note {
    pub comment: String,
    pub expressions: Vec<EExpression>,
}

impl Note {
    pub fn new(comment: String, expressions: &[EExpression]) -> Self {
        Self {
            comment,
            expressions: expressions.to_vec(),
        }
    }
}
impl Rule for Note {
    fn get_comment(&self) -> &str {
        &self.comment
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

/// The Conflict Rule <A conflicts with B>
/// Conflicts evaluate as true if both expressions evaluate as true
#[derive(Default, Clone)]
pub struct Conflict {
    pub comment: String,
    // todo: make first atomic?
    pub expression_a: Option<EExpression>,
    pub expression_b: Option<EExpression>,
}
impl Conflict {
    pub fn new(comment: String, expression_a: EExpression, expression_b: EExpression) -> Self {
        Self {
            comment,
            expression_a: Some(expression_a),
            expression_b: Some(expression_b),
        }
    }
}
impl Rule for Conflict {
    fn get_comment(&self) -> &str {
        &self.comment
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

/// The Require Rule <A requires B>
/// Requires evaluates as true if A is true and B is not true
#[derive(Default, Clone)]
pub struct Requires {
    pub comment: String,
    // todo: make first atomic?
    pub expression_a: Option<EExpression>,
    pub expression_b: Option<EExpression>,
}
impl Requires {
    pub fn new(comment: String, expression_a: EExpression, expression_b: EExpression) -> Self {
        Self {
            comment,
            expression_a: Some(expression_a),
            expression_b: Some(expression_b),
        }
    }
}
impl Rule for Requires {
    fn get_comment(&self) -> &str {
        &self.comment
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
