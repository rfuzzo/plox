////////////////////////////////////////////////////////////////////////
// EXPRESSIONS
////////////////////////////////////////////////////////////////////////

use serde::{Deserialize, Serialize};

use crate::wild_contains;

// An expression may be evaluated against a load order
pub trait TExpression {
    fn eval(&self, items: &[String]) -> bool;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Expression {
    Atomic(Atomic),
    ALL(ALL),
    ANY(ANY),
    NOT(NOT),
    DESC(DESC),
    SIZE(SIZE),
}

// pass-through
impl TExpression for Expression {
    fn eval(&self, items: &[String]) -> bool {
        match self {
            Expression::Atomic(x) => x.eval(items),
            Expression::ALL(x) => x.eval(items),
            Expression::ANY(x) => x.eval(items),
            Expression::NOT(x) => x.eval(items),
            Expression::DESC(x) => x.eval(items),
            Expression::SIZE(x) => x.eval(items),
        }
    }
}
// conversions
impl From<Atomic> for Expression {
    fn from(val: Atomic) -> Self {
        Expression::Atomic(val)
    }
}
impl From<ALL> for Expression {
    fn from(val: ALL) -> Self {
        Expression::ALL(val)
    }
}
impl From<ANY> for Expression {
    fn from(val: ANY) -> Self {
        Expression::ANY(val)
    }
}
impl From<NOT> for Expression {
    fn from(val: NOT) -> Self {
        Expression::NOT(val)
    }
}
impl From<DESC> for Expression {
    fn from(val: DESC) -> Self {
        Expression::DESC(val)
    }
}
impl From<SIZE> for Expression {
    fn from(val: SIZE) -> Self {
        Expression::SIZE(val)
    }
}

////////////////////////////////////////////////////////////////////////
// IMPLEMENTATIONS
////////////////////////////////////////////////////////////////////////

////////////////////////////////////////////////////////////////////////
// ATOMIC

/// The atomic expression (EXISTS)
/// atomics evaluate as true if the input list contains the item
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Atomic {
    pub item: String,
}

impl Atomic {
    pub fn get_item(&self) -> String {
        self.item.to_owned()
    }
}
impl TExpression for Atomic {
    /// atomics evaluate as true if the input list contains the item
    fn eval(&self, items: &[String]) -> bool {
        wild_contains(items, &self.item).is_some()
    }
}

impl From<&str> for Atomic {
    fn from(value: &str) -> Self {
        Atomic { item: value.into() }
    }
}
impl From<String> for Atomic {
    fn from(value: String) -> Self {
        Atomic { item: value }
    }
}

////////////////////////////////////////////////////////////////////////
// ALL

/// The ALL expression
/// ALL evaluates as true if all expressions evaluate as true
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ALL {
    pub expressions: Vec<Expression>,
}
impl ALL {
    pub fn new(expressions: Vec<Expression>) -> Self {
        Self { expressions }
    }
}
impl TExpression for ALL {
    /// ALL evaluates as true if all expressions evaluate as true
    fn eval(&self, items: &[String]) -> bool {
        let mut r = true;
        self.expressions
            .iter()
            .map(|e| e.eval(items))
            .for_each(|e| {
                r = r && e;
            });
        r
    }
}

////////////////////////////////////////////////////////////////////////
// ANY

/// The ANY expression (OR)
/// ANY evaluates as true if any expressions evaluates as true
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ANY {
    pub expressions: Vec<Expression>,
}
impl ANY {
    pub fn new(expressions: Vec<Expression>) -> Self {
        Self { expressions }
    }
}
impl TExpression for ANY {
    // ANY evaluate as true if any expressions evaluates as true
    fn eval(&self, items: &[String]) -> bool {
        let mut r = false;
        self.expressions
            .iter()
            .map(|e| e.eval(items))
            .for_each(|e| {
                r = r || e;
            });
        r
    }
}

////////////////////////////////////////////////////////////////////////
// NOT

/// The NOT expression
/// NOT evaluates as true if the wrapped expression evaluates as true
#[derive(Debug, Serialize, Deserialize)]
pub struct NOT {
    pub expression: Box<Expression>,
}
impl NOT {
    pub fn new(expression: Expression) -> Self {
        Self {
            expression: Box::new(expression),
        }
    }
}
impl TExpression for NOT {
    // NOT evaluates as true if the wrapped expression evaluates as true
    fn eval(&self, items: &[String]) -> bool {
        !self.expression.eval(items)
    }
}
impl Clone for NOT {
    fn clone(&self) -> Self {
        Self {
            expression: self.expression.clone(),
        }
    }
}

////////////////////////////////////////////////////////////////////////
// DESC

/// The DESC expression
/// TODO DESC evaluates as true if the expression evaluates as true
#[derive(Debug, Serialize, Deserialize)]
pub struct DESC {
    pub description: String,
    pub expression: Box<Expression>,
    pub is_negated: bool,
}
impl DESC {
    pub fn new(expression: Expression, description: String, is_negated: bool) -> Self {
        Self {
            expression: Box::new(expression),
            description,
            is_negated,
        }
    }
}
impl TExpression for DESC {
    // TODO DESC evaluates as true if the expression evaluates as true
    fn eval(&self, items: &[String]) -> bool {
        self.expression.eval(items)
    }
}
impl Clone for DESC {
    fn clone(&self) -> Self {
        Self {
            expression: self.expression.clone(),
            description: self.description.clone(),
            is_negated: self.is_negated,
        }
    }
}

////////////////////////////////////////////////////////////////////////
// SIZE

/// The SIZE expression
/// TODO SIZE evaluates as true if the expression evaluates as true
#[derive(Debug, Serialize, Deserialize)]
pub struct SIZE {
    pub size: usize,
    pub expression: Box<Expression>,
    pub is_negated: bool,
}
impl SIZE {
    pub fn new(expression: Expression, size: usize, is_negated: bool) -> Self {
        Self {
            expression: Box::new(expression),
            size,
            is_negated,
        }
    }
}
impl TExpression for SIZE {
    // TODO SIZE evaluates as true if the expression evaluates as true
    fn eval(&self, items: &[String]) -> bool {
        self.expression.eval(items)
    }
}
impl Clone for SIZE {
    fn clone(&self) -> Self {
        Self {
            expression: self.expression.clone(),
            size: self.size,
            is_negated: self.is_negated,
        }
    }
}
