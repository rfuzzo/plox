////////////////////////////////////////////////////////////////////////
// EXPRESSIONS
////////////////////////////////////////////////////////////////////////

// An expression may be evaluated against a load order
pub trait TExpression {
    fn eval(&self, items: &[String]) -> bool;
}

#[derive(Clone, Debug)]
pub enum Expression {
    Atomic(Atomic),
    ALL(ALL),
    ANY(ANY),
    NOT(NOT),
}

// pass-through
impl TExpression for Expression {
    fn eval(&self, items: &[String]) -> bool {
        match self {
            Expression::Atomic(x) => x.eval(items),
            Expression::ALL(x) => x.eval(items),
            Expression::ANY(x) => x.eval(items),
            Expression::NOT(x) => x.eval(items),
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

////////////////////////////////////////////////////////////////////////
// IMPLEMENTATIONS
////////////////////////////////////////////////////////////////////////

////////////////////////////////////////////////////////////////////////
// ATOMIC

/// The atomic expression (EXISTS)
/// atomics evaluate as true if the input list contains the item
#[derive(Clone, Debug)]
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
        // TODO wildcards
        items.contains(&self.item)
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
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
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
#[derive(Debug)]
pub struct NOT {
    pub expression: Box<Expression>,
}

impl Clone for NOT {
    fn clone(&self) -> Self {
        Self {
            expression: self.expression.clone(),
        }
    }
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
