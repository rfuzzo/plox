#[cfg(test)]
mod unit_tests {
    use cmop::{topo_sort, ExpressionKind, RuleKind, Rules};

    #[test]
    fn test_cycle() {
        let rules = Rules {
            order: vec![("a", "b"), ("b", "c"), ("d", "e"), ("b", "a")]
                .iter()
                .map(|e| (e.0.to_owned(), e.1.to_owned()))
                .collect(),
        };

        let mods: Vec<String> = vec!["a", "b", "c", "d", "e", "f", "g"]
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
            order: vec![
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

        let mods = vec!["a", "b", "c", "d", "e", "f", "g"]
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

    #[derive(Default)]
    struct Rule {
        pub kind: RuleKind,
        pub expr: Expression,
        pub note: String,
    }

    #[derive(Default)]
    struct Expression {
        pub kind: ExpressionKind,
        pub names: Vec<Expression>,
    }

    impl Expression {
        fn new_from_string(name: String) -> Self {
            Self {
                kind: ExpressionKind::Exists,
                names: vec![Expression::new_from_string(name)],
            }
        }
    }

    /*impl Expression {
        fn new(names: Vec<String>) -> Self {
            Self {
                kind: ExpressionKind::Exists,
                names,
            }
        }
    }*/

    #[test]
    fn test_notes() {
        let mods: Vec<String> = vec!["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        let notes: Vec<_> = vec![("a", "some note"), ("c", "some warning!")]
            .iter()
            .map(|e| Rule {
                kind: RuleKind::Note,
                expr: Expression::new_from_string(e.0.into()),
                note: e.1.into(),
            })
            .collect();

        for rule in notes {
            if eval_rule(&rule, &mods) {
                println!("{}", rule.note);
            }
        }
    }

    fn eval_rule(rule: &Rule, mods: &[String]) -> bool {
        match rule.kind {
            RuleKind::None => panic!("invalid rule"),
            RuleKind::Order => panic!("invalid rule"),
            RuleKind::Note => eval(&rule.expr, mods),
        }
    }

    fn eval(expr: &Expression, mods: &[String]) -> bool {
        match expr.kind {
            ExpressionKind::Exists => {
                // there is only one expression and it exists
                mods.contains(expr.names.first().unwrap())
            }
            ExpressionKind::And => {
                // all expressions evaluate as true
                todo!()
            }
            ExpressionKind::Any => {
                // one expression evaluate as true
                todo!()
            }
            ExpressionKind::Not => {
                // expression is not true
                !eval(expr, mods)
            }
        }
    }
}
