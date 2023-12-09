////////////////////////////////////////////////////////////////////////
/// RULES
////////////////////////////////////////////////////////////////////////
use crate::expressions::*;

#[derive(Default)]
pub struct Rules {
    pub order: Vec<(String, String)>,
    pub warnings: Vec<RuleKind>,
}

pub enum RuleKind {
    //Order(Order), // TODO refactor this into a rule?
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

/// The Note Rule <Note for A>
/// Notes simply check the expression and notify the user if eval is true
#[derive(Default, Clone)]
pub struct Note {
    pub comment: String,
    pub expression: Option<EExpression>,
}

impl Note {
    pub fn new(comment: String, expression: EExpression) -> Self {
        Self {
            comment,
            expression: Some(expression),
        }
    }
}
impl Rule for Note {
    fn get_comment(&self) -> &str {
        &self.comment
    }
    /// Notes evaluate as true if the expression evaluates as true
    fn eval(&self, items: &[String]) -> bool {
        if let Some(expr) = &self.expression {
            return expr.eval(items);
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
