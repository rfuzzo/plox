#[cfg(test)]
mod unit_tests {
    use core::panic;
    use std::io::Cursor;

    use cmop::{expressions::*, parser::*, rules::*};

    #[test]
    fn test_tokenize() {
        {
            let input = " a.archive \"mod with spaces.archive\" \"c.archive\"";
            let expected = ["a.archive", "mod with spaces.archive", "c.archive"];
            assert_eq!(expected, tokenize(input.to_owned()).as_slice());
        }

        {
            let input = "a.archive";
            let expected = ["a.archive"];
            assert_eq!(expected, tokenize(input.to_owned()).as_slice());
        }
    }

    ////////////////////////////////////////////////////////////////////////
    // ORDER

    #[test]
    fn test_multiline_order() {
        let input = "[Order]\na.archive\nb.archive\nc.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
    fn test_inline_order() {
        let input = "[Order]a.archive \"b name.archive\" c.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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

    ////////////////////////////////////////////////////////////////////////
    // NOTE

    #[test]
    fn test_inline_note() {
        let input = "[Note message] a.archive b.archive c.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input = "[Note message] a.archive b name.archive c.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input = "[Note message]\na.archive\nb.archive\nc.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input = "[Note]\n message\na.archive\nb.archive\nc.archive";
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input = "[Note]\na b c.archive\nb.archive";
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input = "[Note]\n[ALL a.archive [NOT b.archive]]";
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input = "[Conflict message] a.archive b.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input = "[Conflict message] a.archive b name.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input = "[Conflict message]\na.archive\nb.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input = "[Conflict message]\na.archive\nb.archive\nc.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input = "[Conflict]\nname a.archive\nb.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input = "[Conflict]\nname a.archive\n[ALL b.archive c name.archive]".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input: String = "[Requires message] a.archive b.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input = "[Requires message] a.archive b name.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        let input = "[Requires message]\na.archive\nb.archive".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
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
        test_atomic("a.archive", "a.archive");
        test_atomic("a name.archive", "a name.archive");
    }

    fn test_atomic(input: &str, expected: &str) {
        assert_eq!(
            1,
            parse_expressions(Cursor::new(input.as_bytes()))
                .expect("No expressions parsed")
                .len()
        );
        let e = parse_expression(input).expect("No expressions parsed");
        assert!(is_atomic(&e, expected));
    }

    #[test]
    fn test_all_expr() {
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
        assert_eq!(
            1,
            parse_expressions(Cursor::new(input.as_bytes()))
                .expect("No expressions parsed")
                .len()
        );
        let expr = parse_expression(input).expect("No expressions parsed");
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
        {
            let input = "[ANY [NOT x.archive] archive name.archive\na.archive]".to_owned();
            let reader = Cursor::new(input.as_bytes());
            let expr = parse_expressions(reader).expect("No expressions parsed");
            assert_eq!(1, expr.len());
        }
        {
            let input = "a.archive Assassins Armory - Arrows.archive a.archive c.archive\n[ANY AreaEffectArrows XB Edition.archive\nAreaEffectArrows.archive b.archive\n[NOT x.archive]]".to_owned();
            let reader = Cursor::new(input.as_bytes());
            let expr = parse_expressions(reader).expect("No expressions parsed");
            assert_eq!(5, expr.len());
        }
    }
}
