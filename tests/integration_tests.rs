#[cfg(test)]
mod integration_tests {
    use plox::{parser::*, *};

    use test_log::test;

    #[test]
    fn test_read_mods() {
        let mods_path = "./tests/modlist.txt";
        assert_eq!(
            read_file_as_list(mods_path),
            vec![
                "a.archive",
                "b.archive",
                "c.archive",
                "d.archive",
                "e.archive"
            ]
        )
    }

    #[test]
    fn test_verify_rules() {
        let rules =
            parse_rules_from_path("./tests/plox/rules_order.txt").expect("rule parse failed");
        let order = get_order_from_rules(&rules);
        let mods = debug_get_mods_from_rules(&order);

        assert!(topo_sort(&mods, &order).is_ok(), "rules contain a cycle")
    }

    #[test]
    fn test_verify_mlox_base_rules() {
        let rules = parse_rules_from_path("./tests/mlox/mlox_base.txt").expect("rule parse failed");
        let order = get_order_from_rules(&rules);

        // debug
        let mods = debug_get_mods_from_rules(&order);

        assert!(topo_sort(&mods, &order).is_ok(), "rules contain a cycle")
    }

    #[test]
    fn test_verify_mlox_user_rules() {
        let rules = parse_rules_from_path("./tests/mlox/mlox_user.txt").expect("rule parse failed");
        let order = get_order_from_rules(&rules);

        // debug
        let mods = debug_get_mods_from_rules(&order);

        assert!(topo_sort(&mods, &order).is_ok(), "rules contain a cycle")
    }

    #[test]
    fn test_parse_rules() {
        let rules =
            parse_rules_from_path("./tests/plox/rules_note.txt").expect("rule parse failed");
        assert_eq!(14, rules.len());
    }

    #[test]
    fn test_gather_mods() {
        let root_path = "./tests";

        match gather_mods(&root_path) {
            Ok(mods) => {
                assert_eq!(
                    mods,
                    vec![
                        "a.archive".to_owned(),
                        "b.archive".into(),
                        "c.archive".into()
                    ]
                )
            }
            Err(_) => panic!("gethering mods failed"),
        }
    }
}
