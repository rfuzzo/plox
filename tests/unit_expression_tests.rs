#[cfg(test)]
mod unit_expressions_tests {
    use cmop::{Atomic, Expression, ALL, ANY, NOT};

    #[test]
    fn evaluate_all() {
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        let mut expr = ALL::new(vec![
            Box::new(Atomic::from("a")),
            Box::new(Atomic::from("b")),
        ]);
        assert!(expr.eval(&mods));

        expr = ALL::new(vec![
            Box::new(Atomic::from("a")),
            Box::new(Atomic::from("x")),
        ]);
        assert!(!expr.eval(&mods));
    }

    #[test]
    fn evaluate_any() {
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        let mut expr = ANY::new(vec![
            Box::new(Atomic::from("a")),
            Box::new(Atomic::from("x")),
        ]);
        assert!(expr.eval(&mods));

        expr = ANY::new(vec![
            Box::new(Atomic::from("y")),
            Box::new(Atomic::from("x")),
        ]);
        assert!(!expr.eval(&mods));
    }

    #[test]
    fn evaluate_not() {
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        let mut expr = NOT::new(Box::new(Atomic::from("x")));
        assert!(expr.eval(&mods));

        expr = NOT::new(Box::new(Atomic::from("a")));
        assert!(!expr.eval(&mods));
    }
}
