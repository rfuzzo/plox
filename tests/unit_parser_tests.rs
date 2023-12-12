#[cfg(test)]
mod unit_tests {
    use std::io::Cursor;

    use cmop::{expressions::*, parser::*, rules::*};

    #[test]
    fn test_tokenize() {
        {
            let input = " a \"mod with spaces.archive\" \"c\"";
            let expected = ["a", "mod with spaces.archive", "c"];
            assert_eq!(expected, tokenize(input.to_owned()).as_slice());
        }

        {
            let input = "a";
            let expected = ["a"];
            assert_eq!(expected, tokenize(input.to_owned()).as_slice());
        }
    }

    ////////////////////////////////////////////////////////////////////////
    /// NOTE
    ////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_inline_note() {
        let input = "[Note message] a b c".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a", "b", "c"];
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
        let input = "[Note message]\na\nb\nc".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a", "b", "c"];
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
        let input = "[Note]\n message\na\nb\nc";
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("message", rule.get_comment());

        let names = ["a", "b", "c"];
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
        let input = "[Note]\na b c.archive\nb";
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("", rule.get_comment());

        let names = ["a b c.archive", "b"];
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
        let input = "[Note]\n[ALL a [NOT b]]";
        let reader = BufReader::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!("", rule.get_comment());
    }
     */

    ////////////////////////////////////////////////////////////////////////
    /// ORDER
    ////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_multiline_order() {
        let input = "[Order]\na\nb\nc".to_owned();
        let reader = Cursor::new(input.as_bytes());

        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
        assert_eq!(2, rules.len());

        let mut rule = rules.first().expect("No rules found");
        if let Rule::Order(n) = rule {
            assert_eq!("a", n.name_a.as_str());
            assert_eq!("b", n.name_b.as_str());
        }

        rule = rules.get(1).expect("No rules found");
        if let Rule::Order(n) = rule {
            assert_eq!("b", n.name_a.as_str());
            assert_eq!("c", n.name_b.as_str());
        }
    }
}
