#[cfg(test)]
mod unit_tests {
    use plox::{expressions::*, PluginData};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    const A: &str = "a.esp";
    const B: &str = "b.esp";
    const C: &str = "c.esp";
    const D: &str = "d.esp";
    const E: &str = "e.esp";
    const F: &str = "f.esp";
    const X: &str = "x.esp";
    const Y: &str = "y.esp";

    fn e(str: &str) -> Expression {
        Atomic::from(str).into()
    }

    fn get_mods() -> Vec<PluginData> {
        [A, B, C, D, E, F]
            .iter()
            .map(|e| PluginData::new(e.to_string(), 0))
            .collect::<Vec<_>>()
    }

    #[test]
    fn evaluate_all() {
        init();

        // [ALL] is true if A and B are true
        {
            let expr = ALL::new(vec![e(A), e(B)]);
            assert!(expr.eval(&get_mods()).is_some());
        }

        // [ALL] is false if A is true and B is not true
        {
            let expr = ALL::new(vec![e(A), e(X)]);
            assert!(expr.eval(&get_mods()).is_none());
        }

        // [ALL] is false if A is not true and B is true
        {
            let expr = ALL::new(vec![e(X), e(A)]);
            assert!(expr.eval(&get_mods()).is_none());
        }

        // [ALL] is false if A is not true and B is not true
        {
            let expr = ALL::new(vec![e(X), e(Y)]);
            assert!(expr.eval(&get_mods()).is_none());
        }
    }

    #[test]
    fn evaluate_any() {
        init();

        // [ANY] is true if A and B are true
        {
            let expr = ANY::new(vec![e(A), e(B)]);
            assert!(expr.eval(&get_mods()).is_some());
        }

        // [ANY] is true if A is true and B is not true
        {
            let expr = ANY::new(vec![e(A), e(X)]);
            assert!(expr.eval(&get_mods()).is_some());
        }

        // [ANY] is true if A is not true and B is true
        {
            let expr = ANY::new(vec![e(X), e(A)]);
            assert!(expr.eval(&get_mods()).is_some());
        }

        // [ANY] is false if A is not true and B is not true
        {
            let expr = ANY::new(vec![e(X), e(Y)]);
            assert!(expr.eval(&get_mods()).is_none());
        }
    }

    #[test]
    fn evaluate_not() {
        init();

        // [NOT] is true if A is not true
        {
            let expr = NOT::new(e(X));
            assert!(expr.eval(&get_mods()).is_some());
        }

        // [NOT] is false if A is true
        {
            let expr = NOT::new(e(A));
            assert!(expr.eval(&get_mods()).is_none());
        }
    }

    #[test]
    fn evaluate_size() {
        init();

        let mods = [A, B, C, D, E, F]
            .iter()
            .enumerate()
            .map(|(i, e)| PluginData::new(e.to_string(), (i + 1) as u64))
            .collect::<Vec<_>>();

        // [SIZE] is true if the plugin size matches the given size
        {
            let expr = SIZE::new(Atomic::from(A), 1_u64, false);
            assert!(expr.eval(&mods).is_some());
        }

        // [SIZE] is true if the plugin size does not matches the given size and is negated
        {
            let expr = SIZE::new(Atomic::from(A), 2_u64, true);
            assert!(expr.eval(&mods).is_some());
        }

        // [SIZE] is false if the plugin size does not match the given size
        {
            let expr = SIZE::new(Atomic::from(A), 2_u64, false);
            assert!(expr.eval(&mods).is_none());
        }
    }

    #[test]
    fn evaluate_nested() {
        init();

        // check that (a and x) are not present in the modlist
        {
            let nested = ALL::new(vec![e(A), e(X)]);
            let expr = NOT::new(nested.into());
            assert!(expr.eval(&get_mods()).is_some());
        }
        // check that (a and b) are not present in the modlist
        {
            let nested = ALL::new(vec![e(A), e(B)]);
            let expr = NOT::new(nested.into());
            assert!(expr.eval(&get_mods()).is_none()); // should fail
        }

        // check that (a and b) are present and that either (x and y) are not present
        {
            let nested1 = ALL::new(vec![e(A), e(B)]);
            let nested2 = NOT::new(ANY::new(vec![e(X), e(Y)]).into());
            let expr = ALL::new(vec![nested1.into(), nested2.into()]);
            assert!(expr.eval(&get_mods()).is_some());
        }

        // check that (a and b) are present and that either (x and y) are present
        {
            let nested1 = ALL::new(vec![e(A), e(B)]);
            let nested2 = ANY::new(vec![e(A), e(Y)]);
            let expr = ALL::new(vec![nested1.into(), nested2.into()]);
            assert!(expr.eval(&get_mods()).is_some());
        }
    }
}
