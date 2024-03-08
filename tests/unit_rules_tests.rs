#[cfg(test)]
mod unit_tests {
    use plox::{expressions::*, rules::*, sorter::new_stable_sorter};

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

    fn get_mods() -> Vec<String> {
        [A, B, C, D, E, F].iter().map(|e| (*e).into()).collect()
    }

    #[test]
    fn test_notes() {
        init();

        // test that [Note] evaluates as true when a mod is present
        {
            let mut rule = Note::new("".into(), &[e(A)]);
            assert!(rule.eval(&get_mods()));
        }

        // test that [Note] evaluates as true when both mods is present
        {
            let mut rule = Note::new("".into(), &[e(A), e(B)]);
            assert!(rule.eval(&get_mods()));
        }

        // test that [Note] evaluates as true when one of two mods is present
        {
            let mut rule = Note::new("".into(), &[e(A), e(X)]);
            assert!(rule.eval(&get_mods()));
        }

        // test that [Note] evaluates as false when a mod is not present
        {
            let mut rule = Note::new("".into(), &[e(X)]);
            assert!(!rule.eval(&get_mods()));
        }

        // test that [Note] evaluates as false when a mod is not present
        {
            let mut rule = Note::new("".into(), &[e(X), e(Y)]);
            assert!(!rule.eval(&get_mods()));
        }
    }

    #[test]
    fn test_conflicts() {
        init();

        // test that [Conflict] evaluates as true when both mods are present
        {
            let mut rule = Conflict::new("".into(), &[e(A), e(B)]);
            assert!(rule.eval(&get_mods()));
        }

        // test that the order doesn't matter
        {
            let mut rule = Conflict::new("".into(), &[e(B), e(A)]);
            assert!(rule.eval(&get_mods()));
        }

        // test that [Conflict] doesn't evaluate as true when one is missing
        {
            let mut rule = Conflict::new("".into(), &[e(B), e(X)]);
            assert!(!rule.eval(&get_mods()));
        }

        // test that the order doesn't matter
        {
            let mut rule = Conflict::new("".into(), &[e(X), e(B)]);
            assert!(!rule.eval(&get_mods()));
        }

        // test that [Conflict] doesn't evaluate as true when both are missing
        {
            let mut rule = Conflict::new("".into(), &[e(X), e(Y)]);
            assert!(!rule.eval(&get_mods()));
        }
    }

    #[test]
    fn test_requires() {
        init();

        // test that [Requires] evaluates as true when A is true and B is not
        {
            let mut rule = Requires::new("".into(), e(A), e(X));
            assert!(rule.eval(&get_mods()));
        }

        // test that the order does matter
        {
            let mut rule = Requires::new("".into(), e(X), e(A));
            assert!(!rule.eval(&get_mods()));
        }

        // test that [Requires] evaluates as false when both mods are missing
        {
            let mut rule = Requires::new("".into(), e(X), e(Y));
            assert!(!rule.eval(&get_mods()));
        }

        // test that [Requires] evaluates as false when both mods are there
        {
            let mut rule = Requires::new("".into(), e(A), e(B));
            assert!(!rule.eval(&get_mods()));
        }
    }

    #[test]
    fn test_patch() {
        init();

        // test that [Patch] evaluates as true when A is true and B is not: mod is there, but patch is missing
        {
            let mut rule = Patch::new("".into(), e(A), e(X));
            assert!(rule.eval(&get_mods()));
        }

        // test that [Patch] evaluates as true when B is true and A is not: patch is there, but mod is missing
        {
            let mut rule = Patch::new("".into(), e(X), e(A));
            assert!(rule.eval(&get_mods()));
        }

        // test that [Patch] evaluates as false when both mods are missing
        {
            let mut rule = Patch::new("".into(), e(X), e(Y));
            assert!(!rule.eval(&get_mods()));
        }

        // test that [Patch] evaluates as false when both mods are there
        {
            let mut rule = Patch::new("".into(), e(A), e(B));
            assert!(!rule.eval(&get_mods()));
        }
    }

    #[test]
    fn test_nearstart() {
        // check one gets sorted at the start
        {
            let nearstart = NearStart::new(vec![D.to_string()]);
            let mods = get_mods();
            let order_rules: Vec<EOrderRule> = vec![EOrderRule::NearStart(nearstart)];

            match new_stable_sorter().topo_sort(&mods, &order_rules) {
                Ok(result) => {
                    // check for A,B,C,D,E,F -> D,A,B,C,E,F
                    assert_eq!(
                        vec![
                            D.to_string(),
                            A.to_string(),
                            B.to_string(),
                            C.to_string(),
                            E.to_string(),
                            F.to_string()
                        ],
                        result
                    );
                }
                Err(e) => panic!("Error: {}", e),
            }
        }

        // check that two get sorted at the start
        {
            let nearstart = NearStart::new(vec![B.to_string(), D.to_string()]);
            let mods = get_mods();
            let order_rules: Vec<EOrderRule> = vec![EOrderRule::NearStart(nearstart)];

            match new_stable_sorter().topo_sort(&mods, &order_rules) {
                Ok(result) => {
                    // check for A,B,C,D,E,F -> B,D,A,C,E,F
                    assert_eq!(
                        vec![
                            B.to_string(),
                            D.to_string(),
                            A.to_string(),
                            C.to_string(),
                            E.to_string(),
                            F.to_string()
                        ],
                        result
                    );
                }
                Err(e) => panic!("Error: {}", e),
            }
        }

        // check that two get sorted at the start
        {
            let nearstart = NearStart::new(vec![D.to_string(), B.to_string()]);
            let mods = get_mods();
            let order_rules: Vec<EOrderRule> = vec![EOrderRule::NearStart(nearstart)];

            match new_stable_sorter().topo_sort(&mods, &order_rules) {
                Ok(result) => {
                    // check for A,B,C,D,E,F -> D,B,A,C,E,F
                    assert_eq!(
                        vec![
                            D.to_string(),
                            B.to_string(),
                            A.to_string(),
                            C.to_string(),
                            E.to_string(),
                            F.to_string()
                        ],
                        result
                    );
                }
                Err(e) => panic!("Error: {}", e),
            }
        }
    }

    #[test]
    fn test_order_case() {
        {
            let mods = vec![
                "a.esp".to_string(),
                "b.ESP".to_string(),
                "c.esp".to_string(),
            ];
            let order: Order = Order::new(vec!["b.esp".to_string(), "a.esp".to_string()]);
            let order_rules: Vec<EOrderRule> = vec![order.into()];

            match new_stable_sorter().topo_sort(&mods, &order_rules) {
                Ok(result) => {
                    // check for A,B,C -> B,A,C
                    assert_eq!(
                        vec![
                            "b.ESP".to_string(),
                            "a.esp".to_string(),
                            "c.esp".to_string()
                        ],
                        result
                    );
                }
                Err(e) => panic!("Error: {}", e),
            }
        }
    }

    #[test]
    fn test_order() {
        {
            let mods = get_mods();
            let order: Order = Order::new(vec![D.to_string(), A.to_string()]);
            let order_rules: Vec<EOrderRule> = vec![order.into()];

            match new_stable_sorter().topo_sort(&mods, &order_rules) {
                Ok(result) => {
                    // check for A,B,C,D,E,F -> D,A,B,C,E,F
                    assert_eq!(
                        vec![
                            D.to_string(),
                            A.to_string(),
                            B.to_string(),
                            C.to_string(),
                            E.to_string(),
                            F.to_string(),
                        ],
                        result
                    );
                }
                Err(e) => panic!("Error: {}", e),
            }
        }

        {
            let mods = get_mods();
            let order: Order = Order::new(vec![D.to_string(), A.to_string()]);
            let order_rules: Vec<EOrderRule> = vec![order.into()];

            match new_stable_sorter().topo_sort(&mods, &order_rules) {
                Ok(result) => {
                    // check for A,B,C,D,E,F -> A,B,C,E,F,D
                    assert_ne!(
                        vec![
                            A.to_string(),
                            D.to_string(),
                            B.to_string(),
                            C.to_string(),
                            E.to_string(),
                            F.to_string(),
                        ],
                        result
                    );
                }
                Err(e) => panic!("Error: {}", e),
            }
        }
    }

    #[test]
    fn test_nearend() {
        // check one gets sorted at the start
        {
            let mods = get_mods();
            let nearend = NearEnd::new(vec![D.to_string()]);
            let order_rules: Vec<EOrderRule> = vec![EOrderRule::NearEnd(nearend)];

            match new_stable_sorter().topo_sort(&mods, &order_rules) {
                Ok(result) => {
                    // check for A,B,C,D,E,F -> A,B,C,E,F,D
                    assert_eq!(
                        vec![
                            A.to_string(),
                            B.to_string(),
                            C.to_string(),
                            E.to_string(),
                            F.to_string(),
                            D.to_string(),
                        ],
                        result
                    );
                }
                Err(e) => panic!("Error: {}", e),
            }
        }

        // check that two get sorted at the start
        {
            let nearend = NearEnd::new(vec![B.to_string(), D.to_string()]);
            let mods = get_mods();
            let order_rules: Vec<EOrderRule> = vec![EOrderRule::NearEnd(nearend)];

            match new_stable_sorter().topo_sort(&mods, &order_rules) {
                Ok(result) => {
                    // check for A,B,C,D,E,F -> A,C,E,F,D,B
                    assert_eq!(
                        vec![
                            A.to_string(),
                            C.to_string(),
                            E.to_string(),
                            F.to_string(),
                            D.to_string(),
                            B.to_string(),
                        ],
                        result
                    );
                }
                Err(e) => panic!("Error: {}", e),
            }
        }

        // check that two get sorted at the start
        {
            let nearend = NearEnd::new(vec![D.to_string(), B.to_string()]);
            let mods = get_mods();
            let order_rules: Vec<EOrderRule> = vec![EOrderRule::NearEnd(nearend)];

            match new_stable_sorter().topo_sort(&mods, &order_rules) {
                Ok(result) => {
                    // check for A,B,C,D,E,F -> A,C,E,F,B,D
                    assert_eq!(
                        vec![
                            A.to_string(),
                            C.to_string(),
                            E.to_string(),
                            F.to_string(),
                            B.to_string(),
                            D.to_string(),
                        ],
                        result
                    );
                }
                Err(e) => panic!("Error: {}", e),
            }
        }
    }

    // Nested tests
    #[test]
    fn test_nested() {
        init();

        // test that [ALL] is true if A and B is true
        {
            let mut rule = Note::new("".into(), &[ALL::new(vec![e(A), e(B)]).into()]);
            assert!(rule.eval(&get_mods()));
        }

        // test that [ALL] is false if A is true and B is not true
        {
            let mut rule = Note::new("".into(), &[ALL::new(vec![e(A), e(X)]).into()]);
            assert!(!rule.eval(&get_mods()));
        }

        // test that [ALL] is false if A is not true and B is true
        {
            let mut rule = Note::new("".into(), &[ALL::new(vec![e(X), e(A)]).into()]);
            assert!(!rule.eval(&get_mods()));
        }

        // test that [ALL] is false if A is not true and B is not true
        {
            let mut rule = Note::new("".into(), &[ALL::new(vec![e(X), e(Y)]).into()]);
            assert!(!rule.eval(&get_mods()));
        }

        // test that [ANY] is true if A or B is true
        {
            let mut rule = Note::new("".into(), &[ANY::new(vec![e(A), e(X)]).into()]);
            assert!(rule.eval(&get_mods()));
        }

        // test that [ANY] is true if A and B are not true
        {
            let mut rule = Note::new("".into(), &[ANY::new(vec![e(Y), e(X)]).into()]);
            assert!(!rule.eval(&get_mods()));
        }
    }
}
