////////////////////////////////////////////////////////////////////////
// EXPRESSIONS
////////////////////////////////////////////////////////////////////////

use std::fmt::Display;

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
    VER(VER),
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
            Expression::VER(x) => x.eval(items),
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
impl From<VER> for Expression {
    fn from(val: VER) -> Self {
        Expression::VER(val)
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

/// TODO The Desc predicate is a special predicate that matches strings in the header of a plugin with regular expressions.
#[derive(Debug, Serialize, Deserialize)]
pub struct DESC {
    pub expression: Box<Expression>,
    pub description: String,
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

/// TODO The Size predicate is a special predicate that matches the filesize of the plugin
#[derive(Debug, Serialize, Deserialize)]
pub struct SIZE {
    pub expression: Box<Expression>,
    pub size: usize,
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

////////////////////////////////////////////////////////////////////////
// VER

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum EVerOperator {
    Less,
    Equal,
    Greater,
}

impl Display for EVerOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EVerOperator::Less => write!(f, "<"),
            EVerOperator::Equal => write!(f, "="),
            EVerOperator::Greater => write!(f, ">"),
        }
    }
}

/// TODO The Ver predicate is a special predicate that first tries to match the version number string stored in the plugin header,
/// and if that fails it tries to match the version number from the plugin filename.
/// If a version number is found, it can be used in a comparison.
/// Syntax: [VER operator version plugin.esp]
#[derive(Debug, Serialize, Deserialize)]
pub struct VER {
    pub expression: Box<Expression>,
    pub operator: EVerOperator,
    pub version: String,
}
impl VER {
    pub fn new(expression: Expression, operator: EVerOperator, version: String) -> Self {
        Self {
            expression: Box::new(expression),
            operator,
            version,
        }
    }
}
impl TExpression for VER {
    fn eval(&self, items: &[String]) -> bool {
        self.expression.eval(items)
    }
}
impl Clone for VER {
    fn clone(&self) -> Self {
        Self {
            expression: self.expression.clone(),
            operator: self.operator,
            version: self.version.clone(),
        }
    }
}
