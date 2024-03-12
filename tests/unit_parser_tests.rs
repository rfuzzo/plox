#[cfg(test)]
mod unit_tests {
    use core::panic;
    use std::io::Cursor;

    use plox::{expressions::Expression, rules::TWarningRule, *};

    fn init() {
        let env = env_logger::Env::default()
            .default_filter_or(log_level_to_str(ELogLevel::Debug))
            .default_write_style_or("always");
        let _ = env_logger::Builder::from_env(env).is_test(true).try_init();
    }

    ////////////////////////////////////////////////////////////////////////
    // ORDER

    #[test]
    fn test_order() {
        init();

        let tokens = [
            ("a.archive", "b.archive", "c.archive"),
            ("a with a whitespace.archive", "b.archive", "c.archive"),
        ];

        for token in tokens {
            let a = token.0;
            let b = token.1;
            let c = token.2;

            let inputs = [
                format!("[Order]{a} {b} {c}"),
                format!("[Order]\n{a}\n{b}\n{c}"),
                format!("[Order]{a}\n{b}\n{c}"),
                format!("[Order]{a}\n{b}\n;\"ignored.esp\"\n{c}"),
                format!("[Order]{a}; with a comment\n{b} {c}"),
                format!("[Order]{a} {b} {c} ; with a comment"),
                format!("[Order]{a}; with a comment\n{b}\n{c}"),
                format!("[Order]; with a comment\n{a}\n{b}\n{c}"),
            ];

            for input in inputs {
                let input = input.to_lowercase();
                let reader = Cursor::new(input.as_bytes());

                let rules = parser::new_cyberpunk_parser()
                    .parse_rules_from_reader(reader)
                    .expect("Failed to parse rule")
                    .into_iter()
                    .filter_map(order)
                    .collect::<Vec<_>>();
                assert_eq!(1, rules.len());

                let n = rules.first().expect("No rules found");
                assert_eq!(a, n.names[0]);
                assert_eq!(b, n.names[1]);
                assert_eq!(c, n.names[2]);
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////
    // NEARSTART

    #[test]
    fn test_nearstart() {
        init();

        let inputs = [
            "[Nearstart message] a.esp b.esp".to_owned(),
            "[Nearstart message] a.esp\nb.esp".to_owned(),
            "[Nearstart]; with a comment\na.esp\nb.esp".to_owned(),
        ];

        for input in inputs {
            let input = input.to_lowercase();
            let reader = Cursor::new(input.as_bytes());

            let rules = parser::new_tes3_parser()
                .parse_rules_from_reader(reader)
                .expect("Failed to parse rule")
                .into_iter()
                .filter_map(nearstart)
                .collect::<Vec<_>>();
            assert_eq!(1, rules.len());
            let n = rules.first().expect("No rules found");

            assert_eq!(2, n.names.len());

            assert_eq!("a.esp", n.names[0]);
            assert_eq!("b.esp", n.names[1]);
        }
    }

    ////////////////////////////////////////////////////////////////////////
    // NEAREND

    #[test]
    fn test_nearend() {
        init();

        let inputs = [
            "[Nearend message] a.esp b.esp",
            "[Nearend message] a.esp\nb.esp",
            "[Nearend]; with a comment\na.esp\nb.esp",
        ];

        for input in inputs {
            let input = input.to_lowercase();
            let reader = Cursor::new(input.as_bytes());

            let rules = parser::new_tes3_parser()
                .parse_rules_from_reader(reader)
                .expect("Failed to parse rule")
                .into_iter()
                .filter_map(nearend)
                .collect::<Vec<_>>();

            assert_eq!(1, rules.len());
            let n = rules.first().expect("No rules found");

            assert_eq!(2, n.names.len());

            assert_eq!("a.esp", n.names[0]);
            assert_eq!("b.esp", n.names[1]);
        }
    }

    ////////////////////////////////////////////////////////////////////////
    // NOTE

    #[test]
    fn test_note() {
        init();

        let tokens = [
            ("a.archive", "b.archive", "c.archive"),
            ("a with a whitespace.archive", "b.archive", "c.archive"),
        ];

        for token in tokens {
            let a = token.0;
            let b = token.1;
            let c = token.2;

            let inputs = [
                format!("[Note message]{a} {b} {c}"),
                format!("[Note message]{a}\n{b}\n{c}"),
                format!("[Note message]\n{a}\n{b}\n{c}"),
                format!("[Note message]{a}; with a comment\n{b}\n{c}"),
                format!("[Note message]{a} {b} {c} ; with a comment"),
                format!("[Note message]{a}; with a comment\n{b} {c}"),
                format!("[Note message]; with a comment\n{a}\n{b}\n{c}"),
                //format!("[Note message]{a} \"{b}\" {c}"),
                //format!("[Note message]{a}\n\"{b}\"\n{c}"),
                // format!("[Note message]\n\"{a}\"\n{b}\n\"{c}\""),
                // format!("[Note message]\n\"{a}\"\n{b}\n\"{c}\""),
            ];

            for input in inputs {
                let input = input.to_lowercase();
                let reader = Cursor::new(input.as_bytes());

                let rules = parser::new_cyberpunk_parser()
                    .parse_rules_from_reader(reader)
                    .expect("Failed to parse rule")
                    .into_iter()
                    .filter_map(note)
                    .collect::<Vec<_>>();

                assert_eq!(1, rules.len());

                let n = rules.first().expect("No rules found");
                assert_eq!("message", n.get_comment());
                assert_eq!(3, n.expressions.len());

                assert!(is_atomic(&n.expressions[0], a));
                assert!(is_atomic(&n.expressions[1], b));
                assert!(is_atomic(&n.expressions[2], c));
            }
        }
    }

    #[test]
    fn test_note_nested() {
        init();

        let input = "[Note]\n[ALL a.archive [NOT b.archive]]".to_lowercase();
        let reader = Cursor::new(input.as_bytes());

        let rules = parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule")
            .into_iter()
            .filter_map(note)
            .collect::<Vec<_>>();
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("", rule.get_comment());
        assert_eq!(1, rule.expressions.len());
    }

    ////////////////////////////////////////////////////////////////////////
    // CONFLICT

    #[test]
    fn test_conflict() {
        let tokens = [
            ("a.archive", "b.archive", "c.archive"),
            //("a with a whitespace.archive", "b.archive", "c.archive"),
        ];

        for token in tokens {
            let a = token.0;
            let b = token.1;
            //let c = token.2;

            let inputs = [
                format!("[Conflict] ; some comment\n\tmessage\n{a}\n{b}"),
                // format!("[Conflict message] {a} {b}"),
                // format!("[Conflict message] {a}\n{b}"),
                // format!("[Conflict message]{a} {b}"),
                // format!("[Conflict message]{a}\n{b}"),
                // format!("[Conflict message]\n{a}\n{b}"),
                // format!("[Conflict message]\n{a}\n{b}"),
                // format!("[Conflict message] {a}; with a comment\n{b}"),
                // format!("[Conflict message] {a}\n{b}; and comment"),
                // format!("[Conflict message]{a}; with a comment\n{b}"),
                // format!("[Conflict message]{a}\n{b}; and comment"),
            ];

            for input in inputs {
                let input = input.to_lowercase();
                let reader = Cursor::new(input.as_bytes());

                let rules = parser::new_cyberpunk_parser()
                    .parse_rules_from_reader(reader)
                    .expect("Failed to parse rule")
                    .into_iter()
                    .filter_map(conflict)
                    .collect::<Vec<_>>();
                assert_eq!(1, rules.len());
                let n = rules.first().expect("No rules found");
                assert_eq!("message", n.get_comment());

                assert!(is_atomic(&n.expressions[0], a));
                assert!(is_atomic(&n.expressions[1], b));
            }
        }
    }

    #[test]
    fn test_conflict_nested() {
        init();

        let input = "[Conflict]\nname a.archive\n[ALL b.archive c name.archive]".to_lowercase();
        let reader = Cursor::new(input.as_bytes());

        let rules = parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule")
            .into_iter()
            .filter_map(conflict)
            .collect::<Vec<_>>();
        assert_eq!(1, rules.len());
        let n = rules.first().expect("No rules found");
        assert_eq!("", n.get_comment());

        assert!(is_atomic(&n.expressions[0], "name a.archive"));
        assert!(is_all(&n.expressions[1]));
    }

    ////////////////////////////////////////////////////////////////////////
    // REQUIRES

    #[test]
    fn test_requires() {
        let tokens = [
            ("a.archive", "b.archive", "c.archive"),
            ("a with a whitespace.archive", "b.archive", "c.archive"),
        ];

        for token in tokens {
            let a = token.0;
            let b = token.1;
            //let c = token.2;

            let inputs = [
                format!("[Requires message] {a} {b}"),
                format!("[Requires message] {a}\n{b}"),
                format!("[Requires message]{a} {b}"),
                format!("[Requires message]{a}\n{b}"),
                format!("[Requires message]\n{a}\n{b}"),
                format!("[Requires message] {a}; with a comment\n{b}"),
                format!("[Requires message] {a}\n{b}; and comment"),
                format!("[Requires message]{a}; with a comment\n{b}"),
                format!("[Requires message]{a}\n{b}; and comment"),
            ];

            for input in inputs {
                let input = input.to_lowercase();
                let reader = Cursor::new(input.as_bytes());

                let rules = parser::new_cyberpunk_parser()
                    .parse_rules_from_reader(reader)
                    .expect("Failed to parse rule")
                    .into_iter()
                    .filter_map(requires)
                    .collect::<Vec<_>>();

                assert_eq!(1, rules.len());
                let n = rules.first().expect("No rules found");
                assert_eq!("message", n.get_comment());

                assert!(n.expression_a.is_some());
                assert!(n.expression_b.is_some());

                assert!(is_atomic(&n.expression_a.clone().unwrap(), a));
                assert!(is_atomic(&n.expression_b.clone().unwrap(), b));
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////
    // PATCH

    #[test]
    fn test_patch() {
        let tokens = [
            ("a.archive", "b.archive", "c.archive"),
            ("a with a whitespace.archive", "b.archive", "c.archive"),
        ];

        for token in tokens {
            let a = token.0;
            let b = token.1;

            let inputs = [
                format!("[Patch message] {a} {b}"),
                format!("[Patch message] {a}\n{b}"),
                format!("[Patch message]{a} {b}"),
                format!("[Patch message]{a}\n{b}"),
                format!("[Patch message]\n{a}\n{b}"),
                format!("[Patch message]\n{a}\n{b}"),
                format!("[Patch message] {a}; with a comment\n{b}"),
                format!("[Patch message] {a}\n{b}; and comment"),
            ];

            for input in inputs {
                let input = input.to_lowercase();
                let reader = Cursor::new(input.as_bytes());

                let rules = parser::new_cyberpunk_parser()
                    .parse_rules_from_reader(reader)
                    .expect("Failed to parse rule")
                    .into_iter()
                    .filter_map(patch)
                    .collect::<Vec<_>>();

                assert_eq!(1, rules.len());
                let n = rules.first().expect("No rules found");
                assert_eq!("message", n.get_comment());

                assert!(n.expression_a.is_some());
                assert!(n.expression_b.is_some());

                assert!(is_atomic(&n.expression_a.clone().unwrap(), a));
                assert!(is_atomic(&n.expression_b.clone().unwrap(), b));
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////
    // EXPRESSIONS
    ////////////////////////////////////////////////////////////////////////

    // Atomic

    #[test]
    fn test_atomic_expr() {
        init();

        let inputs = [
            "[]Siege at Firemoth.esp",
            "[Official]Siege at Firemoth.esp",
            "a.esp",
            "a name.esp",
        ];

        for a in inputs {
            test_atomic(a, a);
        }
    }

    fn test_atomic(input: &str, expected: &str) {
        let parser = parser::new_tes3_parser();

        let exprs = parser
            .parse_expressions(Cursor::new(input.as_bytes()))
            .expect("No expressions parsed");
        assert_eq!(1, exprs.len());

        let e = parser
            .parse_expression(input, false)
            .expect("No expressions parsed");
        assert!(is_atomic(&e, expected));
    }

    // ALL

    #[test]
    fn test_all_expr() {
        init();

        let inputs = [
            ("a.archive", "b.archive"),
            ("a.archive", "b with spaces.archive"),
            ("a name.archive", "b.archive"),
        ];

        for (a, b) in inputs {
            test_all(
                format!("[ALL {a} {b}]").to_lowercase().as_str(),
                [a, b].to_vec(),
            );
        }
    }

    fn test_all(input: &str, expected: Vec<&str>) {
        let parser = parser::new_cyberpunk_parser();
        assert_eq!(
            1,
            parser
                .parse_expressions(Cursor::new(input.as_bytes()))
                .expect("No expressions parsed")
                .len()
        );
        let expr = parser
            .parse_expression(input, true)
            .expect("No expressions parsed");
        if let Expression::ALL(b) = expr {
            assert_eq!(expected.len(), b.expressions.len());
            for (i, e) in b.expressions.iter().enumerate() {
                assert!(is_atomic(e, expected[i]));
            }
        } else {
            panic!("wrong type");
        }
    }

    // DESC
    #[test]
    fn test_desc_expr() {
        init();

        let inputs = [
            ("/regex/", "a.archive"),
            ("/regex with spaces/", "a.archive"),
            ("/regex/", "a some name.archive"),
            ("/regex with spaces/", "a some name.archive"),
        ];

        for (a, b) in inputs {
            test_desc(
                format!("[DESC {a} {b}]").to_lowercase().as_str(),
                [a, b].to_vec(),
            );
        }

        let inputs = [
            ("!/regex/", "a.archive"),
            ("!/regex with spaces/", "a.archive"),
            ("!/regex/", "a some name.archive"),
            ("!/regex with spaces/", "a some name.archive"),
        ];

        for (a, b) in inputs {
            test_desc_neg(
                format!("[DESC {a} {b}]").to_lowercase().as_str(),
                [a, b].to_vec(),
            );
        }
    }

    fn test_desc(input: &str, expected: Vec<&str>) {
        let parser = parser::new_cyberpunk_parser();

        assert_eq!(
            1,
            parser
                .parse_expressions(Cursor::new(input.as_bytes()))
                .expect("No expressions parsed")
                .len()
        );

        let expr = parser
            .parse_expression(input, true)
            .expect("No expressions parsed");

        if let Expression::DESC(e) = expr {
            assert!(is_atomic(&e.expression.into(), expected[1]));
            assert_eq!(format!("/{}/", e.regex), expected[0]);
            assert!(!e.is_negated);
        } else {
            panic!("wrong type");
        }
    }

    fn test_desc_neg(input: &str, expected: Vec<&str>) {
        let parser = parser::new_cyberpunk_parser();

        assert_eq!(
            1,
            parser
                .parse_expressions(Cursor::new(input.as_bytes()))
                .expect("No expressions parsed")
                .len()
        );

        let expr = parser
            .parse_expression(input, true)
            .expect("No expressions parsed");

        if let Expression::DESC(e) = expr {
            assert!(is_atomic(&e.expression.into(), expected[1]));
            assert_eq!(format!("!/{}/", e.regex), expected[0]);
            assert!(e.is_negated);
        } else {
            panic!("wrong type");
        }
    }

    // SIZE
    #[test]
    fn test_size_expr() {
        init();

        let inputs = [("30881", "a.archive"), ("30881", "a some name.archive")];

        for (a, b) in inputs {
            test_size(
                format!("[SIZE {a} {b}]").to_lowercase().as_str(),
                [a, b].to_vec(),
            );
        }

        let inputs = [("!30881", "a.archive"), ("!30881", "a some name.archive")];

        for (a, b) in inputs {
            test_size_neg(
                format!("[SIZE {a} {b}]").to_lowercase().as_str(),
                [a, b].to_vec(),
            );
        }
    }

    fn test_size(input: &str, expected: Vec<&str>) {
        let parser = parser::new_cyberpunk_parser();

        assert_eq!(
            1,
            parser
                .parse_expressions(Cursor::new(input.as_bytes()))
                .expect("No expressions parsed")
                .len()
        );

        let expr = parser
            .parse_expression(input, true)
            .expect("No expressions parsed");

        if let Expression::SIZE(e) = expr {
            assert_eq!(format!("{}", e.size), expected[0]);
            assert!(!e.is_negated);
        } else {
            panic!("wrong type");
        }
    }

    fn test_size_neg(input: &str, expected: Vec<&str>) {
        let parser = parser::new_cyberpunk_parser();

        assert_eq!(
            1,
            parser
                .parse_expressions(Cursor::new(input.as_bytes()))
                .expect("No expressions parsed")
                .len()
        );

        let expr = parser
            .parse_expression(input, true)
            .expect("No expressions parsed");

        if let Expression::SIZE(e) = expr {
            assert_eq!(format!("!{}", e.size), expected[0]);
            assert!(e.is_negated);
        } else {
            panic!("wrong type");
        }
    }

    // VER
    #[test]
    fn test_ver_expr() {
        init();

        let inputs = [
            ("<", "1.51", "a.archive"),
            ("<", "1.51", "a some name.archive"),
            (">", "1.51", "a.archive"),
            (">", "1.51", "a some name.archive"),
            ("=", "1.51", "a.archive"),
            ("=", "1.51", "a some name.archive"),
        ];

        for (a, b, c) in inputs {
            test_ver(
                format!("[VER {a} {b} {c}]").to_lowercase().as_str(),
                [a, format!("{b}.0").as_str(), c].to_vec(),
            );
        }
    }

    fn test_ver(input: &str, expected: Vec<&str>) {
        let parser = parser::new_cyberpunk_parser();

        assert_eq!(
            1,
            parser
                .parse_expressions(Cursor::new(input.as_bytes()))
                .expect("No expressions parsed")
                .len()
        );

        let expr = parser
            .parse_expression(input, true)
            .expect("No expressions parsed");

        if let Expression::VER(e) = expr {
            assert_eq!(format!("{}", e.operator), expected[0]);
            assert_eq!(format!("{}", e.version), expected[1]);
            assert!(is_atomic(&e.expression.into(), expected[2]));
        } else {
            panic!("wrong type");
        }
    }

    // Helpers

    fn is_atomic(e: &Expression, expected: &str) -> bool {
        if let Expression::Atomic(b) = e {
            assert_eq!(expected, b.get_item().as_str());
            true
        } else {
            panic!("wrong type");
        }
    }

    fn is_all(e: &Expression) -> bool {
        if let Expression::ALL(_b) = e {
            true
        } else {
            panic!("wrong type");
        }
    }

    #[test]
    fn test_multiline_expr() {
        init();

        let parser = parser::new_cyberpunk_parser();

        {
            let input = "[ANY [NOT x.archive] archive name.archive\na.archive]".to_lowercase();
            let reader = Cursor::new(input.as_bytes());
            let expr = parser
                .parse_expressions(reader)
                .expect("No expressions parsed");
            assert_eq!(1, expr.len());
        }
        {
            let input = "a.archive Assassins Armory - Arrows.archive a.archive c.archive\n[ANY AreaEffectArrows XB Edition.archive\nAreaEffectArrows.archive b.archive\n[NOT x.archive]]".to_lowercase();
            let reader = Cursor::new(input.as_bytes());
            let expr = parser
                .parse_expressions(reader)
                .expect("No expressions parsed");
            assert_eq!(5, expr.len());
        }
    }
}
