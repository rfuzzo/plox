#[cfg(test)]
mod unit_tests {
    use core::panic;
    use std::io::Cursor;

    use plox::{
        expressions::*,
        parser::{self},
        rules::*,
    };

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn note(f: ERule) -> Option<Note> {
        match f {
            ERule::Rule(EWarningRule::Note(n)) => Some(n),
            _ => None,
        }
    }

    fn conflict(f: ERule) -> Option<Conflict> {
        match f {
            ERule::Rule(EWarningRule::Conflict(n)) => Some(n),
            _ => None,
        }
    }
    fn requires(f: ERule) -> Option<Requires> {
        match f {
            ERule::Rule(EWarningRule::Requires(n)) => Some(n),
            _ => None,
        }
    }
    fn patch(f: ERule) -> Option<Patch> {
        match f {
            ERule::Rule(EWarningRule::Patch(n)) => Some(n),
            _ => None,
        }
    }

    // order
    fn order(f: ERule) -> Option<Order> {
        match f {
            ERule::EOrderRule(EOrderRule::Order(o)) => Some(o),
            _ => None,
        }
    }
    fn nearstart(f: ERule) -> Option<NearStart> {
        match f {
            ERule::EOrderRule(EOrderRule::NearStart(o)) => Some(o),
            _ => None,
        }
    }
    fn nearend(f: ERule) -> Option<NearEnd> {
        match f {
            ERule::EOrderRule(EOrderRule::NearEnd(o)) => Some(o),
            _ => None,
        }
    }

    #[test]
    fn test_tokenize() {
        init();

        let parser = parser::new_cyberpunk_parser();

        {
            let input = "a.archive my e3.archive.archive";
            let expected = ["a.archive", "my e3.archive.archive"];
            assert_eq!(expected, parser.tokenize(input.to_owned()).as_slice());
        }

        {
            let input = " a.archive \"mod with spaces.archive\" b.archive";
            let expected = ["a.archive", "mod with spaces.archive", "b.archive"];
            assert_eq!(expected, parser.tokenize(input.to_owned()).as_slice());
        }

        {
            let input = " a.archive \"mod with spaces.archive\" \"c.archive\"";
            let expected = ["a.archive", "mod with spaces.archive", "c.archive"];
            assert_eq!(expected, parser.tokenize(input.to_owned()).as_slice());
        }

        {
            let input = "a mod with spaces.archive";
            let expected = ["a mod with spaces.archive"];
            assert_eq!(expected, parser.tokenize(input.to_owned()).as_slice());
        }

        {
            let input = "a.archive";
            let expected = ["a.archive"];
            assert_eq!(expected, parser.tokenize(input.to_owned()).as_slice());
        }
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
                format!("[Order]{a}; with a comment\n{b} {c}"),
                format!("[Order]{a} {b} {c} ; with a comment"),
                format!("[Order]{a} \"{b}\" {c}"),
                format!("[Order]{a}\n{b}\n{c}"),
                //format!("[Order]{a}; with a comment\n{b}\n{c}"),
                format!("[Order]{a}\n\"{b}\"\n{c}"),
                format!("[Order]\n{a}\n{b}\n{c}"),
                //format!("[Order]; with a comment\n{a}\n{b}\n{c}"),
                format!("[Order]\n\"{a}\"\n{b}\n\"{c}\""),
            ];

            for input in inputs {
                let reader = Cursor::new(input.as_bytes());

                let rules = parser::new_cyberpunk_parser()
                    .parse_rules_from_reader(reader)
                    .expect("Failed to parse rule")
                    .into_iter()
                    .filter_map(order)
                    .collect::<Vec<_>>();
                assert_eq!(2, rules.len());

                let mut n = rules.first().expect("No rules found");
                assert_eq!(a, n.name_a.as_str());
                assert_eq!(b, n.name_b.as_str());

                n = rules.get(1).expect("No rules found");
                assert_eq!(b, n.name_a.as_str());
                assert_eq!(c, n.name_b.as_str());
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

        let input: String = "[Nearend message] a.esp b.esp".to_owned();
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

    ////////////////////////////////////////////////////////////////////////
    // NOTE

    #[test]
    fn test_inline_note() {
        init();

        let input = "[Note message] a.archive b.archive c.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let parser = parser::new_cyberpunk_parser();
        let rules = parser
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule")
            .into_iter()
            .filter_map(note)
            .collect::<Vec<_>>();

        assert_eq!(1, rules.len());
        let n = rules.first().expect("No rules found");
        assert_eq!("message", n.get_comment());

        let names = ["a.archive", "b.archive", "c.archive"];
        assert_eq!(3, n.expressions.len());
        for (i, e) in n.expressions.iter().enumerate() {
            if let Expression::Atomic(a) = e {
                assert_eq!(names[i], a.get_item().as_str());
            }
        }
    }

    #[test]
    fn test_inline_note2() {
        init();

        let input = "[Note message] a.archive b name.archive c.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let parser = parser::new_cyberpunk_parser();
        let rules = parser
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule")
            .into_iter()
            .filter_map(note)
            .collect::<Vec<_>>();
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b name.archive", "c.archive"];
        assert_eq!(3, rule.expressions.len());
        for (i, e) in rule.expressions.iter().enumerate() {
            if let Expression::Atomic(a) = e {
                assert_eq!(names[i], a.get_item().as_str());
            }
        }
    }

    #[test]
    fn test_multiline_note() {
        init();

        let input = "[Note message]\na.archive\nb.archive\nc.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule")
            .into_iter()
            .filter_map(note)
            .collect::<Vec<_>>();
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b.archive", "c.archive"];
        assert_eq!(3, rule.expressions.len());
        for (i, e) in rule.expressions.iter().enumerate() {
            if let Expression::Atomic(a) = e {
                assert_eq!(names[i], a.get_item().as_str());
            }
        }
    }

    #[test]
    fn test_multiline_note_with_comment() {
        init();

        let input = "[Note]\n message\na.archive\nb.archive\nc.archive";
        let reader = Cursor::new(input.as_bytes());

        let rules = parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule")
            .into_iter()
            .filter_map(note)
            .collect::<Vec<_>>();
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b.archive", "c.archive"];
        assert_eq!(3, rule.expressions.len());
        for (i, e) in rule.expressions.iter().enumerate() {
            if let Expression::Atomic(a) = e {
                assert_eq!(names[i], a.get_item().as_str());
            }
        }
    }

    #[test]
    fn test_split_note() {
        init();

        let input = "[Note]\na b c.archive\nb.archive";
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

        let names = ["a b c.archive", "b.archive"];
        assert_eq!(2, rule.expressions.len());
        for (i, e) in rule.expressions.iter().enumerate() {
            if let Expression::Atomic(a) = e {
                assert_eq!(names[i], a.get_item().as_str());
            }
        }
    }

    #[test]
    fn test_nested_note() {
        init();

        let input = "[Note]\n[ALL a.archive [NOT b.archive]]";
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
    fn test_inline_conflict() {
        let tokens = [
            ("a.archive", "b.archive", "c.archive"),
            ("a with a whitespace.archive", "b.archive", "c.archive"),
        ];

        for token in tokens {
            let a = token.0;
            let b = token.1;
            //let c = token.2;

            let inputs = [
                format!("[Conflict message] {a} {b}"),
                format!("[Conflict message]\n{a}\n{b}"),
                format!("[Conflict message] {a}\n{b}"),
            ];

            for input in inputs {
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
    fn test_multiline_conflict_expression() {
        init();

        let input = "[Conflict]\nname a.archive\n[ALL b.archive c name.archive]".to_owned();
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
    fn test_inline_requires() {
        init();

        let input: String = "[Requires message] a.archive b.archive".to_owned();
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

        let names = ["a.archive", "b.archive"];
        assert!(n.expression_a.is_some());
        assert!(n.expression_b.is_some());

        assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
        assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
    }

    #[test]
    fn test_inline_requires_whitespace() {
        init();

        let input = "[Requires message] a.archive b name.archive".to_owned();
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

        let names = ["a.archive", "b name.archive"];
        assert!(n.expression_a.is_some());
        assert!(n.expression_b.is_some());

        assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
        assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
    }

    #[test]
    fn test_multiline_requires() {
        init();

        let input = "[Requires message]\na.archive\nb.archive".to_owned();
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

        let names = ["a.archive", "b.archive"];
        assert!(n.expression_a.is_some());
        assert!(n.expression_b.is_some());

        assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
        assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
    }

    ////////////////////////////////////////////////////////////////////////
    // PATCH

    #[test]
    fn test_inline_patch() {
        init();

        let input: String = "[Patch message] patch.esp original.esp".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parser::new_tes3_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule")
            .into_iter()
            .filter_map(patch)
            .collect::<Vec<_>>();
        assert_eq!(1, rules.len());
        let n = rules.first().expect("No rules found");
        assert_eq!("message", n.get_comment());

        let names = ["patch.esp", "original.esp"];
        assert!(n.expression_a.is_some());
        assert!(n.expression_b.is_some());

        assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
        assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
    }

    #[test]
    fn test_inline_patch_whitespace() {
        init();

        let input = "[Patch message] patch.esp original with spaces.esp".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parser::new_tes3_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule")
            .into_iter()
            .filter_map(patch)
            .collect::<Vec<_>>();
        assert_eq!(1, rules.len());
        let n = rules.first().expect("No rules found");
        assert_eq!("message", n.get_comment());

        let names = ["patch.esp", "original with spaces.esp"];
        assert!(n.expression_a.is_some());
        assert!(n.expression_b.is_some());

        assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
        assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
    }

    #[test]
    fn test_multiline_patch() {
        init();

        let input = "[Patch message]\npatch.esp\noriginal.esp".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parser::new_tes3_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule")
            .into_iter()
            .filter_map(patch)
            .collect::<Vec<_>>();
        assert_eq!(1, rules.len());
        let n = rules.first().expect("No rules found");
        assert_eq!("message", n.get_comment());

        let names = ["patch.esp", "original.esp"];
        assert!(n.expression_a.is_some());
        assert!(n.expression_b.is_some());

        assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
        assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
    }

    ////////////////////////////////////////////////////////////////////////
    // EXPRESSIONS
    ////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_atomic_expr() {
        init();

        test_atomic("a.archive", "a.archive");
        test_atomic("a name.archive", "a name.archive");
    }

    fn test_atomic(input: &str, expected: &str) {
        let parser = parser::new_cyberpunk_parser();
        assert_eq!(
            1,
            parser
                .parse_expressions(Cursor::new(input.as_bytes()))
                .expect("No expressions parsed")
                .len()
        );
        let e = parser
            .parse_expression(input)
            .expect("No expressions parsed");
        assert!(is_atomic(&e, expected));
    }

    #[test]
    fn test_all_expr() {
        init();

        test_all(
            "[ALL a.archive     b.archive]",
            ["a.archive", "b.archive"].to_vec(),
        );
        test_all(
            "[ALL a name.archive b.archive]",
            ["a name.archive", "b.archive"].to_vec(),
        );
        test_all(
            "[ALL\na name.archive\nb.archive]",
            ["a name.archive", "b.archive"].to_vec(),
        );
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
            .parse_expression(input)
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
            let input = "[ANY [NOT x.archive] archive name.archive\na.archive]".to_owned();
            let reader = Cursor::new(input.as_bytes());
            let expr = parser
                .parse_expressions(reader)
                .expect("No expressions parsed");
            assert_eq!(1, expr.len());
        }
        {
            let input = "a.archive Assassins Armory - Arrows.archive a.archive c.archive\n[ANY AreaEffectArrows XB Edition.archive\nAreaEffectArrows.archive b.archive\n[NOT x.archive]]".to_owned();
            let reader = Cursor::new(input.as_bytes());
            let expr = parser
                .parse_expressions(reader)
                .expect("No expressions parsed");
            assert_eq!(5, expr.len());
        }
    }
}
