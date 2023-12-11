////////////////////////////////////////////////////////////////////////
/// EXPRESSIONS
////////////////////////////////////////////////////////////////////////

// An expression may be evaluated against a load order
pub trait TExpression {
    fn eval(&self, items: &[String]) -> bool;
    fn parse(&mut self, buffer: Vec<u8>);
}

#[derive(Clone, Debug)]
pub enum Expression {
    Atomic(Atomic),
    ALL(ALL),
    ANY(ANY),
    NOT(NOT),
}
impl Expression {
    fn default() -> Expression {
        // TODO that's kinda dumb
        Atomic::default().into()
    }
}

// pass-through
impl TExpression for Expression {
    fn eval(&self, items: &[String]) -> bool {
        match self {
            Expression::Atomic(atomic) => atomic.eval(items),
            Expression::ALL(all) => all.eval(items),
            Expression::ANY(any) => any.eval(items),
            Expression::NOT(not) => not.eval(items),
        }
    }
    fn parse(&mut self, buffer: Vec<u8>) {
        match self {
            Expression::Atomic(x) => x.parse(buffer),
            Expression::ALL(x) => x.parse(buffer),
            Expression::ANY(x) => x.parse(buffer),
            Expression::NOT(x) => x.parse(buffer),
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
/// IMPLEMENTATIONS
////////////////////////////////////////////////////////////////////////

/// The atomic expression (EXISTS)
/// atomics evaluate as true if the input list contains the item
#[derive(Default, Clone, Debug)]
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

    fn parse(&mut self, buffer: Vec<u8>) {
        // just read the buffer as string
        if let Ok(string) = String::from_utf8(buffer) {
            self.item = string;
        } else {
            // TODO panic
        }
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
#[derive(Default, Clone, Debug)]
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
    fn parse(&mut self, _buffer: Vec<u8>) {
        todo!()
    }
}

/// The ANY expression
/// ANY evaluates as true if any expressions evaluates as true
#[derive(Default, Clone, Debug)]
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
    fn parse(&mut self, _buffer: Vec<u8>) {
        todo!()
    }
}

/// The NOT expression
/// NOT evaluates as true if the wrapped expression evaluates as true
#[derive(Debug)]
pub struct NOT {
    pub expression: Box<Expression>,
}

impl Default for NOT {
    fn default() -> Self {
        Self {
            expression: Box::new(Expression::default()),
        }
    }
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
    fn parse(&mut self, _buffer: Vec<u8>) {
        todo!()
    }
}
