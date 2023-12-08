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
        let mut expr = ALL::new(vec![Atomic::from("a").into(), Atomic::from("b").into()]);
        assert!(expr.eval(&mods));

        // check that a and x exist in my load order
        expr = ALL::new(vec![Atomic::from("a").into(), Atomic::from("x").into()]);
        assert!(!expr.eval(&mods)); // should fail
    }

    #[test]
    fn evaluate_any() {
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        // check that a or x exist in my load order
        let mut expr = ANY::new(vec![Atomic::from("a").into(), Atomic::from("x").into()]);
        assert!(expr.eval(&mods));

        // check that x or y exist in my load order
        expr = ANY::new(vec![Atomic::from("y").into(), Atomic::from("x").into()]);
        assert!(!expr.eval(&mods)); // should fail
    }

    #[test]
    fn evaluate_not() {
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        // check that x is not present in my load order
        let mut expr = NOT::new(Atomic::from("x").into());
        assert!(expr.eval(&mods));

        // check that a is not present in my load order
        expr = NOT::new(Atomic::from("a").into());
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
            let nested = ALL::new(vec![Atomic::from("a").into(), Atomic::from("x").into()]);
            let expr = NOT::new(nested.into());
            assert!(expr.eval(&mods));
        }
        // check that (a and b) are not present in the modlist
        {
            let nested = ALL::new(vec![Atomic::from("a").into(), Atomic::from("b").into()]);
            let expr = NOT::new(nested.into());
            assert!(!expr.eval(&mods)); // should fail
        }

        // check that (a and b) are present and that either (x and y) are not present
        {
            let nested1 = ALL::new(vec![Atomic::from("a").into(), Atomic::from("b").into()]);
            let nested2 =
                NOT::new(ANY::new(vec![Atomic::from("x").into(), Atomic::from("y").into()]).into());
            let expr = ALL::new(vec![nested1.into(), nested2.into()]);
            assert!(expr.eval(&mods));
        }

        // check that (a and b) are present and that either (f and y) are present
        {
            let nested1 = ALL::new(vec![Atomic::from("a").into(), Atomic::from("b").into()]);
            let nested2 = ANY::new(vec![Atomic::from("f").into(), Atomic::from("y").into()]);
            let expr = ALL::new(vec![nested1.into(), nested2.into()]);
            assert!(expr.eval(&mods));
        }
    }
}
