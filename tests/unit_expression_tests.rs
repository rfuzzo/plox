#[cfg(test)]
mod unit_expressions_tests {
    use cmop::{Atomic, Expression, ALL, ANY, NOT};

    #[test]
    fn evaluate_all() {
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        // check that a and b exist in my load order
        let mut expr = ALL::new(vec![
            Box::new(Atomic::from("a")),
            Box::new(Atomic::from("b")),
        ]);
        assert!(expr.eval(&mods));

        // check that a and x exist in my load order
        expr = ALL::new(vec![
            Box::new(Atomic::from("a")),
            Box::new(Atomic::from("x")),
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
            Box::new(Atomic::from("a")),
            Box::new(Atomic::from("x")),
        ]);
        assert!(expr.eval(&mods));

        // check that x or y exist in my load order
        expr = ANY::new(vec![
            Box::new(Atomic::from("y")),
            Box::new(Atomic::from("x")),
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
        let mut expr = NOT::new(Box::new(Atomic::from("x")));
        assert!(expr.eval(&mods));

        // check that a is not present in my load order
        expr = NOT::new(Box::new(Atomic::from("a")));
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
                Box::new(Atomic::from("a")),
                Box::new(Atomic::from("x")),
            ]);
            let expr = NOT::new(Box::new(nested));
            assert!(expr.eval(&mods));
        }
        // check that (a and b) are not present in the modlist
        {
            let nested = ALL::new(vec![
                Box::new(Atomic::from("a")),
                Box::new(Atomic::from("b")),
            ]);
            let expr = NOT::new(Box::new(nested));
            assert!(!expr.eval(&mods)); // should fail
        }

        // check that (a and b) are present and that either (x and y) are not present
        {
            let nested1 = ALL::new(vec![
                Box::new(Atomic::from("a")),
                Box::new(Atomic::from("b")),
            ]);
            let nested2 = NOT::new(Box::new(ANY::new(vec![
                Box::new(Atomic::from("x")),
                Box::new(Atomic::from("y")),
            ])));
            let expr = ALL::new(vec![Box::new(nested1), Box::new(nested2)]);
            assert!(expr.eval(&mods));
        }

        // check that (a and b) are present and that either (f and y) are present
        {
            let nested1 = ALL::new(vec![
                Box::new(Atomic::from("a")),
                Box::new(Atomic::from("b")),
            ]);
            let nested2 = ANY::new(vec![
                Box::new(Atomic::from("f")),
                Box::new(Atomic::from("y")),
            ]);
            let expr = ALL::new(vec![Box::new(nested1), Box::new(nested2)]);
            assert!(expr.eval(&mods));
        }
    }
}
