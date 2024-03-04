#[cfg(test)]
mod unit_tests {
    use plox::{expressions::*, rules::*};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    const A: &str = "a.esp";
    const B: &str = "b.esp";
    const C: &str = "c.esp";
    const D: &str = "d.esp";
    const E: &str = "e.esp";
    const F: &str = "f.esp";
    const X: &str = "x.esp";
    const Y: &str = "y.esp";

    fn e(str: &str) -> Expression {
        Atomic::from(str).into()
    }

    fn get_mods() -> Vec<String> {
        [A, B, C, D, E, F].iter().map(|e| (*e).into()).collect()
    }

    #[test]
    fn test_notes() {
        init();

        // test that [Note] evaluates as true when a mod is present
        {
            let rule = Note::new("".into(), &[e(A)]);
            assert!(rule.eval(&get_mods()));
        }

        // test that [Note] evaluates as true when both mods is present
        {
            let rule = Note::new("".into(), &[e(A), e(B)]);
            assert!(rule.eval(&get_mods()));
        }

        // test that [Note] evaluates as true when one of two mods is present
        {
            let rule = Note::new("".into(), &[e(A), e(X)]);
            assert!(rule.eval(&get_mods()));
        }

        // test that [Note] evaluates as false when a mod is not present
        {
            let rule = Note::new("".into(), &[e(X)]);
            assert!(!rule.eval(&get_mods()));
        }

        // test that [Note] evaluates as false when a mod is not present
        {
            let rule = Note::new("".into(), &[e(X), e(Y)]);
            assert!(!rule.eval(&get_mods()));
        }
    }

    #[test]
    fn test_conflicts() {
        init();

        // test that [Conflict] evaluates as true when both mods are present
        {
            let rule = Conflict::new("".into(), e(A), e(B));
            assert!(rule.eval(&get_mods()));
        }

        // test that the order doesn't matter
        {
            let rule = Conflict::new("".into(), e(B), e(A));
            assert!(rule.eval(&get_mods()));
        }

        // test that [Conflict] doesn't evaluate as true when one is missing
        {
            let rule = Conflict::new("".into(), e(B), e(X));
            assert!(!rule.eval(&get_mods()));
        }

        // test that the order doesn't matter
        {
            let rule = Conflict::new("".into(), e(X), e(B));
            assert!(!rule.eval(&get_mods()));
        }

        // test that [Conflict] doesn't evaluate as true when both are missing
        {
            let rule = Conflict::new("".into(), e(X), e(Y));
            assert!(!rule.eval(&get_mods()));
        }
    }

    #[test]
    fn test_requires() {
        init();

        // test that [Requires] evaluates as true when A is true and B is not
        {
            let rule = Requires::new("".into(), e(A), e(X));
            assert!(rule.eval(&get_mods()));
        }

        // test that the order does matter
        {
            let rule = Requires::new("".into(), e(X), e(A));
            assert!(!rule.eval(&get_mods()));
        }

        // test that [Requires] evaluates as false when both mods are missing
        {
            let rule = Requires::new("".into(), e(X), e(Y));
            assert!(!rule.eval(&get_mods()));
        }

        // test that [Requires] evaluates as false when both mods are there
        {
            let rule = Requires::new("".into(), e(A), e(B));
            assert!(!rule.eval(&get_mods()));
        }
    }

    #[test]
    fn test_patch() {
        init();

        // test that [Patch] evaluates as true when A is true and B is not: mod is there, but patch is missing
        {
            let rule = Patch::new("".into(), e(A), e(X));
            assert!(rule.eval(&get_mods()));
        }

        // test that [Patch] evaluates as true when B is true and A is not: patch is there, but mod is missing
        {
            let rule = Patch::new("".into(), e(X), e(A));
            assert!(rule.eval(&get_mods()));
        }

        // test that [Patch] evaluates as false when both mods are missing
        {
            let rule = Patch::new("".into(), e(X), e(Y));
            assert!(!rule.eval(&get_mods()));
        }

        // test that [Patch] evaluates as false when both mods are there
        {
            let rule = Patch::new("".into(), e(A), e(B));
            assert!(!rule.eval(&get_mods()));
        }
    }

    // Nested tests
    #[test]
    fn test_nested() {
        init();

        // test that [ALL] is true if A and B is true
        {
            let rule = Note::new("".into(), &[ALL::new(vec![e(A), e(B)]).into()]);
            assert!(rule.eval(&get_mods()));
        }

        // test that [ALL] is false if A is true and B is not true
        {
            let rule = Note::new("".into(), &[ALL::new(vec![e(A), e(X)]).into()]);
            assert!(!rule.eval(&get_mods()));
        }

        // test that [ALL] is false if A is not true and B is true
        {
            let rule = Note::new("".into(), &[ALL::new(vec![e(X), e(A)]).into()]);
            assert!(!rule.eval(&get_mods()));
        }

        // test that [ALL] is false if A is not true and B is not true
        {
            let rule = Note::new("".into(), &[ALL::new(vec![e(X), e(Y)]).into()]);
            assert!(!rule.eval(&get_mods()));
        }

        // test that [ANY] is true if A or B is true
        {
            let rule = Note::new("".into(), &[ANY::new(vec![e(A), e(X)]).into()]);
            assert!(rule.eval(&get_mods()));
        }

        // test that [ANY] is true if A and B are not true
        {
            let rule = Note::new("".into(), &[ANY::new(vec![e(Y), e(X)]).into()]);
            assert!(!rule.eval(&get_mods()));
        }
    }
}
