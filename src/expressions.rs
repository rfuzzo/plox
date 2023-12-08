////////////////////////////////////////////////////////////////////////
/// EXPRESSIONS
////////////////////////////////////////////////////////////////////////

/// An expression such as EXISTS, ALL, ANY, NOT
pub trait Expression {
    fn eval(&self, items: &[String]) -> bool;
}

/// The atomic expression (EXISTS)
/// atomics evaluate as true if the input list contains the item
pub struct Atomic {
    pub item: String,
}
impl Expression for Atomic {
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

/// The ALL expression
/// ALL evaluates as true if all expressions evaluate as true
pub struct ALL {
    pub expressions: Vec<Box<dyn Expression>>,
}
impl ALL {
    pub fn new(expressions: Vec<Box<dyn Expression>>) -> Self {
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
}

/// The ANY expression
/// ANY evaluates as true if any expressions evaluates as true
pub struct ANY {
    pub expressions: Vec<Box<dyn Expression>>,
}
impl ANY {
    pub fn new(expressions: Vec<Box<dyn Expression>>) -> Self {
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
}

/// The NOT expression
/// NOT evaluates as true if the wrapped expression evaluates as true
pub struct NOT {
    pub expression: Box<dyn Expression>,
}
impl NOT {
    pub fn new(expression: Box<dyn Expression>) -> Self {
        Self { expression }
    }
}
impl Expression for NOT {
    // NOT evaluates as true if the wrapped expression evaluates as true
    fn eval(&self, items: &[String]) -> bool {
        !self.expression.eval(items)
    }
}
