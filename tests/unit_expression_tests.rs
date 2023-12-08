#[cfg(test)]
mod unit_expressions_tests {
    use cmop::expressions::*;

    #[test]
    fn evaluate_all() {
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        // check that a and b exist in my load order
        let mut expr = ALL::new(vec![
            EExpression::Atomic(Atomic::from("a")),
            EExpression::Atomic(Atomic::from("b")),
        ]);
        assert!(expr.eval(&mods));

        // check that a and x exist in my load order
        expr = ALL::new(vec![
            EExpression::Atomic(Atomic::from("a")),
            EExpression::Atomic(Atomic::from("x")),
        ]);
        assert!(!expr.eval(&mods)); // should fail
    }

    #[test]
    fn evaluate_any() {
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        // check that a or x exist in my load order
        let mut expr = ANY::new(vec![
            EExpression::Atomic(Atomic::from("a")),
            EExpression::Atomic(Atomic::from("x")),
        ]);
        assert!(expr.eval(&mods));

        // check that x or y exist in my load order
        expr = ANY::new(vec![
            EExpression::Atomic(Atomic::from("y")),
            EExpression::Atomic(Atomic::from("x")),
        ]);
        assert!(!expr.eval(&mods)); // should fail
    }

    #[test]
    fn evaluate_not() {
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        // check that x is not present in my load order
        let mut expr = NOT::new(EExpression::Atomic(Atomic::from("x")));
        assert!(expr.eval(&mods));

        // check that a is not present in my load order
        expr = NOT::new(EExpression::Atomic(Atomic::from("a")));
        assert!(!expr.eval(&mods)); // should fail
    }

    #[test]
    fn evaluate_nested() {
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        // check that (a and x) are not present in the modlist
        {
            let nested = ALL::new(vec![
                EExpression::Atomic(Atomic::from("a")),
                EExpression::Atomic(Atomic::from("x")),
            ]);
            let expr = NOT::new(nested.as_expr());
            assert!(expr.eval(&mods));
        }
        // check that (a and b) are not present in the modlist
        {
            let nested = ALL::new(vec![
                EExpression::Atomic(Atomic::from("a")),
                EExpression::Atomic(Atomic::from("b")),
            ]);
            let expr = NOT::new(nested.as_expr());
            assert!(!expr.eval(&mods)); // should fail
        }

        // check that (a and b) are present and that either (x and y) are not present
        {
            let nested1 = ALL::new(vec![
                EExpression::Atomic(Atomic::from("a")),
                EExpression::Atomic(Atomic::from("b")),
            ]);
            let nested2 = NOT::new(
                ANY::new(vec![
                    EExpression::Atomic(Atomic::from("x")),
                    EExpression::Atomic(Atomic::from("y")),
                ])
                .as_expr(),
            );
            let expr = ALL::new(vec![nested1.as_expr(), nested2.as_expr()]);
            assert!(expr.eval(&mods));
        }

        // check that (a and b) are present and that either (f and y) are present
        {
            let nested1 = ALL::new(vec![
                EExpression::Atomic(Atomic::from("a")),
                EExpression::Atomic(Atomic::from("b")),
            ]);
            let nested2 = ANY::new(vec![
                EExpression::Atomic(Atomic::from("f")),
                EExpression::Atomic(Atomic::from("y")),
            ]);
            let expr = ALL::new(vec![nested1.as_expr(), nested2.as_expr()]);
            assert!(expr.eval(&mods));
        }
    }
}
