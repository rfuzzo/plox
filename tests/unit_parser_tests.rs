#[cfg(test)]
mod unit_parser_tests {
    use std::io::BufReader;

    use cmop::{parser::*, rules::*};

    #[test]
    fn test_notes() {
        let input = "[Note message] a b c".to_owned();
        let reader = BufReader::new(input.as_bytes());
        let rules = parse_rules_from_reader(reader).expect("Failed to parse rule");
        assert_eq!(1, rules.len());
        let rule = rules.first().expect("No rules found");
        assert_eq!(rule.get_comment(), "message");
        if let Rule::Note(n) = rule {
            assert_eq!(3, n.expressions.len());
        }
    }

    #[test]
    fn test_tokenize() {
        {
            let input = " a \"mod with spaces.archive\" \"c\"";
            let expected = ["a", "mod with spaces.archive", "c"];
            assert_eq!(expected, tokenize(input.to_owned()).as_slice());
        }
    }
}

/*
Assassins Armory - Arrows.esp
[ANY AreaEffectArrows XB Edition.esp
  AreaEffectArrows.esp]

[ALL A.esp [NOT X.esp]]

[ALL A.esp [NOT X.esp]]
[ANY B.esp 7.esp]
[ALL C.esp elephant.esp potato.esp]

[ALL A.esp
  [NOT X.esp]]

*/
