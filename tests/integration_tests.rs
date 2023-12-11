#[cfg(test)]
mod integration_tests {
    use cmop::{parser::*, *};

    #[test]
    fn test_read_mods() {
        let mods_path = "./tests/modlist.txt";
        assert_eq!(read_file_as_list(mods_path), vec!["a", "b", "c", "d", "e"])
    }

    #[test]
    fn test_parse_rules() {
        assert!(
            parse_rules_from_path("./tests/cmop/rules_order.txt").is_ok(),
            "rules parsing failed"
        );
    }

    #[test]
    fn test_verify_rules() {
        let rules =
            parse_rules_from_path("./tests/cmop/rules_order.txt").expect("rule parse failed");
        let order = get_order_from_rules(&rules);
        let mods = get_mods_from_rules(&order);

        assert!(topo_sort(&mods, &order).is_ok(), "rules contain a cycle")
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
