#[cfg(test)]
mod unit_rules_tests {
    use cmop::{expressions::*, rules::*};

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
                expression: Box::new(Atomic { item: e.0.into() }),
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
                expression_a: Box::new(Atomic { item: "a".into() }),
                expression_b: Box::new(Atomic { item: "b".into() }),
            },
            Conflict {
                comment: "some b".into(),
                expression_a: Box::new(Atomic { item: "b".into() }),
                expression_b: Box::new(Atomic { item: "x".into() }),
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

    #[test]
    fn test_requires() {
        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        let rules: Vec<Conflict> = vec![
            Conflict {
                comment: "some a".into(),
                expression_a: Box::new(Atomic { item: "a".into() }),
                expression_b: Box::new(Atomic { item: "b".into() }),
            },
            Conflict {
                comment: "some b".into(),
                expression_a: Box::new(Atomic { item: "b".into() }),
                expression_b: Box::new(Atomic { item: "x".into() }),
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
