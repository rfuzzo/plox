#[cfg(test)]
mod integration_tests {
    use std::{fs::create_dir_all, io::Write};

    use plox::{parser::*, rules::EOrderRule, sorter::*, *};
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

        let mut parser = new_tes3_parser();
        parser
            .init_from_file("./tests/plox/rules_order.txt")
            .expect("failed rule parsing");

        assert_eq!(5, parser.order_rules.len());

        let mods = debug_get_mods_from_order_rules(&parser.order_rules);

        match new_unstable_sorter().topo_sort(&mods, &parser.order_rules) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("Error: {}", e),
        }

        match new_stable_sorter().topo_sort(&mods, &parser.order_rules) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("Error: {}", e),
        }
    }

    #[test]
    fn test_parse_notes() {
        init();

        {
            let mut parser = new_cyberpunk_parser();
            parser
                .init_from_file("./tests/plox/rules_note_passing.txt")
                .expect("failed rule parsing");
            assert_eq!(10, parser.rules.len());
        }

        {
            let mut parser = new_tes3_parser();
            parser
                .init_from_file("./tests/plox/rules_note_failing.txt")
                .expect("failed rule parsing");
            assert_eq!(0, parser.rules.len());
        }
    }

    #[test]
    fn test_parse_conflicts() {
        init();

        let mut parser = new_tes3_parser();
        parser
            .init_from_file("./tests/plox/rules_conflict.txt")
            .expect("failed rule parsing");
        assert_eq!(5, parser.rules.len());
    }

    #[test]
    fn test_parse_requires() {
        init();

        let mut parser = new_tes3_parser();
        parser
            .init_from_file("./tests/plox/rules_requires.txt")
            .expect("failed rule parsing");
        assert_eq!(1, parser.rules.len());
    }

    #[test]
    fn test_dump_rules() -> std::io::Result<()> {
        init();

        {
            let mut parser = new_tes3_parser();
            parser.init_from_file("./tests/mlox/mlox_base.txt")?;

            {
                let file = std::fs::File::create("base_rules.json").expect("file create failed");
                serde_json::to_writer_pretty(file, &parser.rules).expect("serialize failed");
            }

            {
                let file =
                    std::fs::File::create("base_rules_order.json").expect("file create failed");
                serde_json::to_writer_pretty(file, &parser.order_rules).expect("serialize failed");
            }
        }

        {
            let mut parser = new_tes3_parser();
            parser.init_from_file("./tests/mlox/mlox_user.txt")?;

            {
                let file = std::fs::File::create("user_rules.json").expect("file create failed");
                serde_json::to_writer_pretty(file, &parser.rules).expect("serialize failed");
            }

            {
                let file =
                    std::fs::File::create("user_rules_order.json").expect("file create failed");
                serde_json::to_writer_pretty(file, &parser.order_rules).expect("serialize failed");
            }

            Ok(())
        }
    }

    #[test]
    fn test_dump_display_rules() -> std::io::Result<()> {
        init();

        {
            let mut parser = new_tes3_parser();
            parser.init_from_file("./tests/mlox/mlox_base.txt")?;

            {
                create_dir_all("tmp").expect("could not create dir");
                let mut file =
                    std::fs::File::create("tmp/base_rules.txt").expect("file create failed");
                for rule in parser.rules {
                    writeln!(file, "{}", rule).expect("could not write to file");
                }
            }
        }

        {
            let mut parser = new_tes3_parser();
            parser.init_from_file("./tests/mlox/mlox_user.txt")?;

            {
                create_dir_all("tmp").expect("could not create dir");
                let mut file =
                    std::fs::File::create("tmp/user_rules.txt").expect("file create failed");
                for rule in parser.rules {
                    writeln!(file, "{}", rule).expect("could not write to file");
                }
            }

            Ok(())
        }
    }

    #[allow(dead_code)]
    //TODO disabled for now #[test]
    fn test_mlox_user_rules() -> std::io::Result<()> {
        init();

        let mut parser = new_tes3_parser();
        parser.init_from_file("./tests/mlox/mlox_user.txt")?;

        let mods = debug_get_mods_from_order_rules(&parser.order_rules);

        // let mut rng = thread_rng();
        // mods.shuffle(&mut rng);

        // let file = std::fs::File::create("tmp/mods.json").expect("file create failed");
        // serde_json::to_writer_pretty(file, &mods).expect("serialize failed");

        match new_unstable_sorter().topo_sort(&mods, &parser.order_rules) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("Error: {}", e),
        }

        match new_stable_sorter().topo_sort(&mods, &parser.order_rules) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("Error: {}", e),
        }

        Ok(())
    }

    #[allow(dead_code)]
    //TODO disabled for now #[test]
    fn test_mlox_base_rules() -> std::io::Result<()> {
        init();

        let mut parser = new_tes3_parser();
        parser.init_from_file("./tests/mlox/mlox_base.txt")?;

        let mods = debug_get_mods_from_order_rules(&parser.order_rules);

        // let mut rng = thread_rng();
        // mods.shuffle(&mut rng);

        match new_unstable_sorter().topo_sort(&mods, &parser.order_rules) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("Error: {}", e),
        }

        match new_stable_sorter().topo_sort(&mods, &parser.order_rules) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("Error: {}", e),
        }

        Ok(())
    }

    #[allow(dead_code)]
    //TODO disabled for now #[test]
    fn test_mlox_rules() -> std::io::Result<()> {
        init();

        let mut parser = new_tes3_parser();
        parser.init("./tests/mlox")?;

        let mods = debug_get_mods_from_order_rules(&parser.order_rules);

        // let mut rng = thread_rng();
        // mods.shuffle(&mut rng);

        match new_unstable_sorter().topo_sort(&mods, &parser.order_rules) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("Error: {}", e),
        }

        match new_stable_sorter().topo_sort(&mods, &parser.order_rules) {
            Ok(result) => {
                assert!(
                    checkresult(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("Error: {}", e),
        }

        Ok(())
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

    fn checkresult(result: &[String], order_rules: &[EOrderRule]) -> bool {
        let order = get_ordering_from_order_rules(order_rules);
        let pairs = order;
        for (a, b) in pairs {
            if let Some(results_for_a) = wild_contains(result, &a) {
                if let Some(results_for_b) = wild_contains(result, &b) {
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
