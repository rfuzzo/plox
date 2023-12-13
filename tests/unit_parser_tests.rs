#[cfg(test)]
mod unit_tests {
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

    /*
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
            assert_eq!(3, n.expressions.len());
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
        let reader = BufReader::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("", rule.get_comment());
    }
     */

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

    ////////////////////////////////////////////////////////////////////////
    // CONFLICT

    ////////////////////////////////////////////////////////////////////////
    // REQUIRES

    ////////////////////////////////////////////////////////////////////////
    // EXPRESSIONS
    ////////////////////////////////////////////////////////////////////////

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
