#[cfg(test)]
mod unit_tests {
    use core::panic;
    use std::io::Cursor;

    use plox::{expressions::*, get_order_rules, parser::*, rules::*};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_tokenize() {
        init();

        let parser = Parser::new_cyberpunk_parser();

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
    fn test_multiline_order() {
        init();

        let input = "[Order]\na.archive\nb.archive\nc.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(2, rules.len());

        let mut rule = rules.first().expect("No rules found");
        if let Rule::Order(n) = rule {
            assert_eq!("a.archive", n.name_a.as_str());
            assert_eq!("b.archive", n.name_b.as_str());
        }

        rule = rules.get(1).expect("No rules found");
        if let Rule::Order(n) = rule {
            assert_eq!("b.archive", n.name_a.as_str());
            assert_eq!("c.archive", n.name_b.as_str());
        }
    }

    #[test]
    fn test_multiline_order_with_whitespace() {
        init();

        let input = "[Order]\na.archive\narchive with spaces.archive\nc.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        let order = get_order_rules(&rules);
        assert_eq!(2, order.len());

        let mut rule = rules.first().expect("No rules found");
        if let Rule::Order(n) = rule {
            assert_eq!("a.archive", n.name_a.as_str());
            assert_eq!("archive with spaces.archive", n.name_b.as_str());
        }

        rule = rules.get(1).expect("No rules found");
        if let Rule::Order(n) = rule {
            assert_eq!("archive with spaces.archive", n.name_a.as_str());
            assert_eq!("c.archive", n.name_b.as_str());
        }
    }

    #[test]
    fn test_inline_order() {
        init();

        {
            let input = "[Order]a.archive \"b name.archive\" c.archive".to_owned();
            let reader = Cursor::new(input.as_bytes());

            let rules = Parser::new_cyberpunk_parser()
                .parse_rules_from_reader(reader)
                .expect("Failed to parse rule");
            assert_eq!(2, rules.len());

            let mut rule = rules.first().expect("No rules found");
            if let Rule::Order(n) = rule {
                assert_eq!("a.archive", n.name_a.as_str());
                assert_eq!("b name.archive", n.name_b.as_str());
            }

            rule = rules.get(1).expect("No rules found");
            if let Rule::Order(n) = rule {
                assert_eq!("b name.archive", n.name_a.as_str());
                assert_eq!("c.archive", n.name_b.as_str());
            }
        }

        {
            let input = "[Order]a.archive c.archive ; with a comment".to_owned();
            let reader = Cursor::new(input.as_bytes());

            let rules = Parser::new_cyberpunk_parser()
                .parse_rules_from_reader(reader)
                .expect("Failed to parse rule");
            assert_eq!(1, rules.len());

            let rule = rules.first().expect("No rules found");
            if let Rule::Order(n) = rule {
                assert_eq!("a.archive", n.name_a.as_str());
                assert_eq!("c.archive", n.name_b.as_str());
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////
    // NOTE

    #[test]
    fn test_inline_note() {
        init();

        let input = "[Note message] a.archive b.archive c.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let parser = Parser::new_cyberpunk_parser();
        let rules = parser
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b.archive", "c.archive"];
        if let Rule::Note(n) = rule {
            assert_eq!(3, n.expressions.len());
            for (i, e) in n.expressions.iter().enumerate() {
                if let Expression::Atomic(a) = e {
                    assert_eq!(names[i], a.get_item().as_str());
                }
            }
        }
    }

    #[test]
    fn test_inline_note2() {
        init();

        let input = "[Note message] a.archive b name.archive c.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let parser = Parser::new_cyberpunk_parser();
        let rules = parser
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b name.archive", "c.archive"];
        if let Rule::Note(n) = rule {
            assert_eq!(3, n.expressions.len());
            for (i, e) in n.expressions.iter().enumerate() {
                if let Expression::Atomic(a) = e {
                    assert_eq!(names[i], a.get_item().as_str());
                }
            }
        }
    }

    #[test]
    fn test_multiline_note() {
        init();

        let input = "[Note message]\na.archive\nb.archive\nc.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b.archive", "c.archive"];
        if let Rule::Note(n) = rule {
            assert_eq!(3, n.expressions.len());
            for (i, e) in n.expressions.iter().enumerate() {
                if let Expression::Atomic(a) = e {
                    assert_eq!(names[i], a.get_item().as_str());
                }
            }
        }
    }

    #[test]
    fn test_multiline_note_with_comment() {
        init();

        let input = "[Note]\n message\na.archive\nb.archive\nc.archive";
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b.archive", "c.archive"];
        if let Rule::Note(n) = rule {
            assert_eq!(3, n.expressions.len());
            for (i, e) in n.expressions.iter().enumerate() {
                if let Expression::Atomic(a) = e {
                    assert_eq!(names[i], a.get_item().as_str());
                }
            }
        }
    }

    #[test]
    fn test_split_note() {
        init();

        let input = "[Note]\na b c.archive\nb.archive";
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("", rule.get_comment());

        let names = ["a b c.archive", "b.archive"];
        if let Rule::Note(n) = rule {
            assert_eq!(2, n.expressions.len());
            for (i, e) in n.expressions.iter().enumerate() {
                if let Expression::Atomic(a) = e {
                    assert_eq!(names[i], a.get_item().as_str());
                }
            }
        }
    }

    #[test]
    fn test_nested_note() {
        init();

        let input = "[Note]\n[ALL a.archive [NOT b.archive]]";
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("", rule.get_comment());
        if let Rule::Note(n) = rule {
            assert_eq!(1, n.expressions.len());
        }
    }

    ////////////////////////////////////////////////////////////////////////
    // CONFLICT

    #[test]
    fn test_inline_conflict() {
        init();

        let input = "[Conflict message] a.archive b.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b.archive"];
        if let Rule::Conflict(n) = rule {
            assert!(n.expression_a.is_some());
            assert!(n.expression_b.is_some());

            assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
            assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
        }
    }

    #[test]
    fn test_inline_conflict_whitespace() {
        init();

        let input = "[Conflict message] a.archive b name.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b name.archive"];
        if let Rule::Conflict(n) = rule {
            assert!(n.expression_a.is_some());
            assert!(n.expression_b.is_some());

            assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
            assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
        }
    }

    #[test]
    fn test_multiline_conflict() {
        init();

        let input = "[Conflict message]\na.archive\nb.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b.archive"];
        if let Rule::Conflict(n) = rule {
            assert!(n.expression_a.is_some());
            assert!(n.expression_b.is_some());

            assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
            assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
        }
    }

    #[test]
    fn test_multiline_conflict_overflow() {
        init();

        let input = "[Conflict message]\na.archive\nb.archive\nc.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b.archive"];
        if let Rule::Conflict(n) = rule {
            assert!(n.expression_a.is_some());
            assert!(n.expression_b.is_some());

            assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
            assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
        }
    }

    #[test]
    fn test_multiline_conflict_whitespace() {
        init();

        let input = "[Conflict]\nname a.archive\nb.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("", rule.get_comment());

        let names = ["name a.archive", "b.archive"];
        if let Rule::Conflict(n) = rule {
            assert!(n.expression_a.is_some());
            assert!(n.expression_b.is_some());

            assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
            assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
        }
    }

    #[test]
    fn test_multiline_conflict_expression() {
        init();

        let input = "[Conflict]\nname a.archive\n[ALL b.archive c name.archive]".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("", rule.get_comment());

        let names = ["name a.archive"];
        if let Rule::Conflict(n) = rule {
            assert!(n.expression_a.is_some());
            assert!(n.expression_b.is_some());

            assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
            assert!(is_all(&n.expression_b.clone().unwrap()));
        }
    }

    ////////////////////////////////////////////////////////////////////////
    // REQUIRES

    #[test]
    fn test_inline_requires() {
        init();

        let input: String = "[Requires message] a.archive b.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b.archive"];
        if let Rule::Requires(n) = rule {
            assert!(n.expression_a.is_some());
            assert!(n.expression_b.is_some());

            assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
            assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
        }
    }

    #[test]
    fn test_inline_requires_whitespace() {
        init();

        let input = "[Requires message] a.archive b name.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b name.archive"];
        if let Rule::Requires(n) = rule {
            assert!(n.expression_a.is_some());
            assert!(n.expression_b.is_some());

            assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
            assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
        }
    }

    #[test]
    fn test_multiline_requires() {
        init();

        let input = "[Requires message]\na.archive\nb.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = Parser::new_cyberpunk_parser()
            .parse_rules_from_reader(reader)
            .expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a.archive", "b.archive"];
        if let Rule::Requires(n) = rule {
            assert!(n.expression_a.is_some());
            assert!(n.expression_b.is_some());

            assert!(is_atomic(&n.expression_a.clone().unwrap(), names[0]));
            assert!(is_atomic(&n.expression_b.clone().unwrap(), names[1]));
        }
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
        let parser = Parser::new_cyberpunk_parser();
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
        let parser = Parser::new_cyberpunk_parser();
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

        let parser = Parser::new_cyberpunk_parser();

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
