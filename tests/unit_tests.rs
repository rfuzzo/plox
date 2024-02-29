#[cfg(test)]
mod unit_tests {
    use plox::parser::Parser;
    use plox::{debug_get_mods_from_rules, get_order_rules, topo_sort};
    use plox::{expressions::*, rules::*};

    #[test]
    fn test_cycle() {
        let order = [("a", "b"), ("b", "c"), ("d", "e"), ("b", "a")]
            .iter()
            .map(|e| (e.0.to_owned(), e.1.to_owned()))
            .collect();

        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        assert!(
            topo_sort(&mods, &order, false).is_err(),
            "rules do not contain a cycle"
        )
    }

    #[test]
    fn test_ordering() {
        let order = [
            ("a", "b"),
            ("b", "c"),
            ("d", "e"),
            ("e", "c"),
            ("test.archive", "test2.archive"),
        ]
        .iter()
        .map(|e| (e.0.to_owned(), e.1.to_owned()))
        .collect();

        let mods = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        match topo_sort(&mods, &order, false) {
            Ok(result) => assert!(checkresult(&result, &order), "order is wrong"),
            Err(_) => panic!("rules contain a cycle"),
        }
    }

    #[test]
    fn test_optimized_sort() {
        let rules = Parser::new_tes3_parser()
            .parse_rules_from_path("./tests/mlox/mlox_user.txt")
            .expect("rule parse failed");
        let order = get_order_rules(&rules);

        // debug
        let mods = debug_get_mods_from_rules(&order)
            .into_iter()
            .take(100)
            .collect::<Vec<_>>();

        let full_result = topo_sort(&mods, &order, false).expect("rules contain a cycle");
        let opt_result = topo_sort(&mods, &order, true).expect("opt rules contain a cycle");

        assert_eq!(full_result, opt_result);
    }

    fn checkresult(result: &[String], order: &Vec<(String, String)>) -> bool {
        let pairs = order;
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
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        let rules: Vec<Conflict> = vec![
            Conflict::new(
                "some a".into(),
                Atomic::from("a").into(),
                Atomic::from("b").into(),
            ),
            Conflict::new(
                "some b".into(),
                Atomic::from("a").into(),
                Atomic::from("x").into(),
            ),
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
