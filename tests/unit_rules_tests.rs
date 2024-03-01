#[cfg(test)]
mod unit_tests {
    use plox::{expressions::*, rules::*};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_notes() {
        init();

        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        let rules: Vec<_> = [("a", "some a"), ("c", "some b"), ("x", "some x!")]
            .iter()
            .map(|e| Note::new(e.1.into(), &[Atomic::from(e.0).into()]))
            .collect();

        let mut warnings: Vec<String> = vec![];
        for rule in rules {
            if rule.eval(&mods) {
                warnings.push(rule.get_comment().into());
            }
        }
        let expected: Vec<String> = vec!["some a".to_owned(), "some b".into()];
        assert_eq!(warnings, expected);
    }

    #[test]
    fn test_conflicts() {
        init();

        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        let rule1 = Conflict::new(
            "a conflicts with b".into(),
            Atomic::from("a").into(),
            Atomic::from("b").into(),
        );
        let rule2 = Conflict::new(
            "b conflicts with x".into(),
            Atomic::from("b").into(),
            Atomic::from("x").into(),
        );
        let rules: Vec<Conflict> = vec![
            rule1.clone(), // a conflicts with a
            rule2,         // b conflicts with x
        ];

        let mut warnings: Vec<String> = vec![];
        for rule in rules {
            if rule.eval(&mods) {
                warnings.push(rule.get_comment().into());
            }
        }
        let expected: Vec<String> = vec![rule1.comment];
        assert_eq!(warnings, expected);
    }

    #[test]
    fn test_requires() {
        init();

        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        let rules: Vec<Requires> = vec![
            // a requires b
            Requires::new(
                "a requires b".into(),
                Atomic::from("a").into(),
                Atomic::from("b").into(),
            ),
            // b requires x
            Requires::new(
                "b requires x".into(),
                Atomic::from("b").into(),
                Atomic::from("x").into(),
            ),
            // x requires y
            Requires::new(
                "x requires y".into(),
                Atomic::from("x").into(),
                Atomic::from("y").into(),
            ),
        ];

        let mut warnings: Vec<String> = vec![];
        for rule in rules {
            if rule.eval(&mods) {
                warnings.push(rule.get_comment().into());
            }
        }
        let expected: Vec<String> = vec!["b requires x".to_owned()];
        assert_eq!(warnings, expected);
    }

    // Nested tests

    // ANY
    #[test]
    fn test_any() {
        init();

        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();
        {
            let rule = Note::new(
                "comment".into(),
                &[ALL::new(vec![Atomic::from("a").into(), Atomic::from("b").into()]).into()],
            );
            assert!(rule.eval(&mods));
        }

        {
            let rule = Note::new(
                "comment".into(),
                &[ALL::new(vec![Atomic::from("a").into(), Atomic::from("x").into()]).into()],
            );
            assert!(!rule.eval(&mods));
        }

        {
            let rule = Note::new(
                "comment".into(),
                &[ANY::new(vec![Atomic::from("a").into(), Atomic::from("x").into()]).into()],
            );
            assert!(rule.eval(&mods));
        }

        {
            let rule = Note::new(
                "comment".into(),
                &[ANY::new(vec![Atomic::from("y").into(), Atomic::from("x").into()]).into()],
            );
            assert!(!rule.eval(&mods));
        }
    }
}
