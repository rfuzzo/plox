#[cfg(test)]
mod integration_tests {
    use std::fs::create_dir_all;

    use log::warn;
    use plox::{parser::*, sorter::*, *};
    //use rand::{seq::SliceRandom, thread_rng};

    fn init() {
        let env = env_logger::Env::default()
            .default_filter_or(log_level_to_str(ELogLevel::Debug))
            .default_write_style_or("always");
        let _ = env_logger::Builder::from_env(env).is_test(true).try_init();
    }

    #[test]
    fn test_read_mods() {
        init();

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
    fn test_parse_order() {
        init();

        let rules = new_tes3_parser()
            .parse_rules_from_path("./tests/plox/rules_order.txt")
            .expect("rule parse failed");

        assert_eq!(5, rules.len());
        let order = get_ordering(&rules);

        let mods = debug_get_mods_from_rules(&order);

        if let Ok(result) = new_unstable_sorter().topo_sort(&mods, &order) {
            assert!(checkresult(&result, &order), "stable(true) order is wrong");
        } else {
            panic!("rules contain a cycle")
        }

        if let Ok(result) = new_stable_sorter().topo_sort(&mods, &order) {
            assert!(checkresult(&result, &order), "stable(true) order is wrong");
        } else {
            panic!("rules contain a cycle")
        }
    }

    #[test]
    fn test_parse_notes() {
        init();

        {
            let rules = new_cyberpunk_parser()
                .parse_rules_from_path("./tests/plox/rules_note_passing.txt")
                .expect("rule parse failed");

            assert_eq!(10, rules.len());
        }

        {
            let rules = new_cyberpunk_parser()
                .parse_rules_from_path("./tests/plox/rules_note_failing.txt")
                .expect("rule parse failed");

            assert_eq!(0, rules.len());
        }
    }

    #[test]
    fn test_parse_conflicts() {
        init();

        let rules = new_tes3_parser()
            .parse_rules_from_path("./tests/plox/rules_conflict.txt")
            .expect("rule parse failed");

        assert_eq!(2, rules.len());
    }

    #[test]
    fn test_parse_requires() {
        init();

        let rules = new_tes3_parser()
            .parse_rules_from_path("./tests/plox/rules_requires.txt")
            .expect("rule parse failed");

        assert_eq!(1, rules.len());
    }

    #[test]
    fn test_dump_rules() {
        init();

        {
            let parser = new_tes3_parser();
            let rules = parser
                .parse_rules_from_path("./tests/mlox/mlox_base.txt")
                .expect("rule parse failed");

            let file = std::fs::File::create("base_rules.json").expect("file create failed");
            serde_json::to_writer_pretty(file, &rules).expect("serialize failed");
            {
                create_dir_all("tmp").expect("dir create failed");
                let file =
                    std::fs::File::create("tmp/base_rules_order.json").expect("file create failed");
                serde_json::to_writer_pretty(
                    file,
                    &rules.into_iter().filter_map(order).collect::<Vec<_>>(),
                )
                .expect("serialize failed");
            }
        }

        {
            let parser = new_tes3_parser();
            let rules = parser
                .parse_rules_from_path("./tests/mlox/mlox_user.txt")
                .expect("rule parse failed");

            let file = std::fs::File::create("user_rules.json").expect("file create failed");
            serde_json::to_writer_pretty(file, &rules).expect("serialize failed");
            {
                create_dir_all("tmp").expect("dir create failed");
                let file =
                    std::fs::File::create("tmp/user_rules_order.json").expect("file create failed");
                serde_json::to_writer_pretty(
                    file,
                    &rules.into_iter().filter_map(order).collect::<Vec<_>>(),
                )
                .expect("serialize failed");
            }
        }
    }

    #[test]
    fn scc() {
        init();
        let g = toposort_scc::IndexGraph::from_adjacency_list(&[
            vec![3],
            vec![3, 4],
            vec![4, 7],
            vec![5, 6, 7],
            vec![6],
            vec![],
            vec![],
            vec![],
        ]);

        let mut g2 = g.clone();
        g2.add_edge(6, 2); // cycle [2, 4, 6]

        let scc = g2.scc();
        for (is, s) in scc.iter().enumerate() {
            warn!("s: {}", is);
            for i in s {
                warn!("\t{}", i);
            }
        }
    }

    #[test]
    fn test_mlox_user_order() {
        init();

        let parser = new_tes3_parser();
        let rules = parser
            .parse_rules_from_path("./tests/mlox/mlox_user.txt")
            .expect("rule parse failed");
        let order = rules.into_iter().filter_map(order).collect::<Vec<_>>();
        let ordering = get_ordering_from_orders(&order);
        let mods = debug_get_mods_from_rules(&ordering);

        // let mut rng = thread_rng();
        // mods.shuffle(&mut rng);

        // let file = std::fs::File::create("mods.json").expect("file create failed");
        // serde_json::to_writer_pretty(file, &mods).expect("serialize failed");

        match new_unstable_sorter().topo_sort(&mods, &ordering) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &ordering),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("rules contain a cycle {}", e),
        }

        match new_stable_sorter().topo_sort(&mods, &ordering) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &ordering),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("rules contain a cycle {}", e),
        }
    }

    #[test]
    fn test_mlox_base_rules() {
        init();

        let parser = new_tes3_parser();
        let rules = parser
            .parse_rules_from_path("./tests/mlox/mlox_base.txt")
            .expect("rule parse failed");
        let order = rules.into_iter().filter_map(order).collect::<Vec<_>>();
        let ordering = get_ordering_from_orders(&order);
        let mods = debug_get_mods_from_rules(&ordering);

        // let mut rng = thread_rng();
        // mods.shuffle(&mut rng);

        match new_unstable_sorter().topo_sort(&mods, &ordering) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &ordering),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("rules contain a cycle {}", e),
        }

        match new_stable_sorter().topo_sort(&mods, &ordering) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &ordering),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("rules contain a cycle {}", e),
        }
    }

    #[test]
    fn test_mlox_rules() {
        init();

        let mut parser = new_tes3_parser();
        parser.init("./tests/mlox");
        let rules = parser.order_rules;
        let order = rules.into_iter().filter_map(order2).collect::<Vec<_>>();
        let ordering = get_ordering_from_orders(&order);
        let mods = debug_get_mods_from_rules(&ordering);

        // let mut rng = thread_rng();
        // mods.shuffle(&mut rng);

        match new_unstable_sorter().topo_sort(&mods, &ordering) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &ordering),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("rules contain a cycle {}", e),
        }

        match new_stable_sorter().topo_sort(&mods, &ordering) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &ordering),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("rules contain a cycle {}", e),
        }
    }

    #[test]
    fn test_gather_mods() {
        init();

        let root_path = "./tests";

        let mods = gather_mods(&root_path, ESupportedGame::Cyberpunk);
        assert_eq!(
            mods,
            vec![
                "a.archive".to_owned(),
                "b.archive".into(),
                "c.archive".into()
            ]
        )
    }

    fn checkresult(result: &[String], order: &Vec<(String, String)>) -> bool {
        let pairs = order;
        for (a, b) in pairs {
            if let Some(results_for_a) = wild_contains(result, a) {
                if let Some(results_for_b) = wild_contains(result, b) {
                    for i in &results_for_a {
                        for j in &results_for_b {
                            let pos_a = result.iter().position(|x| x == i).unwrap();
                            let pos_b = result.iter().position(|x| x == j).unwrap();
                            if pos_a > pos_b {
                                return false;
                            }
                        }
                    }
                }
            }
        }

        true
    }
}
