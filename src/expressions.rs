////////////////////////////////////////////////////////////////////////
/// EXPRESSIONS
////////////////////////////////////////////////////////////////////////

pub trait Expression {
    fn eval(&self, items: &[String]) -> bool;
    fn as_expr(&self) -> EExpression;
}

#[derive(Clone)]
pub enum EExpression {
    Atomic(Atomic),
    ALL(ALL),
    ANY(ANY),
    NOT(NOT),
}

impl EExpression {
    pub fn eval(&self, items: &[String]) -> bool {
        match self {
            EExpression::Atomic(atomic) => atomic.eval(items),
            EExpression::ALL(all) => all.eval(items),
            EExpression::ANY(any) => any.eval(items),
            EExpression::NOT(not) => not.eval(items),
        }
    }
}

/// The atomic expression (EXISTS)
/// atomics evaluate as true if the input list contains the item
#[derive(Clone)]
pub struct Atomic {
    pub item: String,
}
impl Expression for Atomic {
    /// atomics evaluate as true if the input list contains the item
    fn eval(&self, items: &[String]) -> bool {
        // TODO wildcards
        items.contains(&self.item)
    }

    fn as_expr(&self) -> EExpression {
        EExpression::Atomic(self.clone())
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

/// The ALL expression
/// ALL evaluates as true if all expressions evaluate as true
#[derive(Clone)]
pub struct ALL {
    pub expressions: Vec<EExpression>,
}
impl ALL {
    pub fn new(expressions: Vec<EExpression>) -> Self {
        Self { expressions }
    }
}
impl Expression for ALL {
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
    fn as_expr(&self) -> EExpression {
        EExpression::ALL(self.clone())
    }
}

/// The ANY expression
/// ANY evaluates as true if any expressions evaluates as true
#[derive(Clone)]
pub struct ANY {
    pub expressions: Vec<EExpression>,
}
impl ANY {
    pub fn new(expressions: Vec<EExpression>) -> Self {
        Self { expressions }
    }
}
impl Expression for ANY {
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

    fn as_expr(&self) -> EExpression {
        EExpression::ANY(self.clone())
    }
}

/// The NOT expression
/// NOT evaluates as true if the wrapped expression evaluates as true
pub struct NOT {
    pub expression: Box<EExpression>,
}

impl Clone for NOT {
    fn clone(&self) -> Self {
        Self {
            expression: self.expression.clone(),
        }
    }
}
impl NOT {
    pub fn new(expression: EExpression) -> Self {
        Self {
            expression: Box::new(expression),
        }
    }
}
impl Expression for NOT {
    // NOT evaluates as true if the wrapped expression evaluates as true
    fn eval(&self, items: &[String]) -> bool {
        !self.expression.eval(items)
    }
    fn as_expr(&self) -> EExpression {
        EExpression::NOT(self.clone())
    }
}