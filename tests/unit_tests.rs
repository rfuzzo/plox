#[cfg(test)]
mod unit_tests {
    use cmop::topo_sort;
    use cmop::{expressions::*, rules::*};

    #[test]
    fn test_cycle() {
        let rules = Rules {
            order: [("a", "b"), ("b", "c"), ("d", "e"), ("b", "a")]
                .iter()
                .map(|e| (e.0.to_owned(), e.1.to_owned()))
                .collect(),
        };

        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        assert!(
            topo_sort(&mods, &rules).is_err(),
            "rules do not contain a cycle"
        )
    }

    #[test]
    fn test_ordering() {
        let rules = Rules {
            order: [
                ("a", "b"),
                ("b", "c"),
                ("d", "e"),
                ("e", "c"),
                ("test.archive", "test2.archive"),
            ]
            .iter()
            .map(|e| (e.0.to_owned(), e.1.to_owned()))
            .collect(),
        };

        let mods = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        match topo_sort(&mods, &rules) {
            Ok(result) => assert!(checkresult(&result, &rules), "order is wrong"),
            Err(_) => panic!("rules contain a cycle"),
        }
    }

    fn checkresult(result: &[String], rules: &Rules) -> bool {
        let pairs = &rules.order;
        for (a, b) in pairs {
            let pos_a = result.iter().position(|x| x == a);
            if pos_a.is_none() {
                continue;
            }
            let pos_b = result.iter().position(|x| x == b);
            if pos_b.is_none() {
                continue;
            }

            if pos_a.unwrap() > pos_b.unwrap() {
                return false;
            }
        }

        true
    }

    #[test]
    fn test_notes() {
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        let rules: Vec<_> = [("a", "some a"), ("c", "some b"), ("x", "some x!")]
            .iter()
            .map(|e| Note {
                comment: e.1.into(),
                expression: Atomic { item: e.0.into() }.into(),
            })
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
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        let rules: Vec<Conflict> = vec![
            Conflict {
                comment: "some a".into(),
                expression_a: Atomic { item: "a".into() }.into(),
                expression_b: Atomic { item: "b".into() }.into(),
            },
            Conflict {
                comment: "some b".into(),
                expression_a: Atomic { item: "b".into() }.into(),
                expression_b: Atomic { item: "x".into() }.into(),
            },
        ];

        let mut warnings: Vec<String> = vec![];
        for rule in rules {
            if rule.eval(&mods) {
                warnings.push(rule.get_comment().into());
            }
        }
        let expected: Vec<String> = vec!["some a".to_owned()];
        assert_eq!(warnings, expected);
    }
}
