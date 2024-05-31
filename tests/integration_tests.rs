#[cfg(test)]
mod integration_tests {
    use std::path::PathBuf;
    use std::{fs::create_dir_all, io::Write};

    use log::warn;
    use plox::{parser::*, sorter::*, *};
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    use semver::Version;

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
        let mods_data = read_file_as_list(mods_path);
        assert_eq!(
            mods_data
                .iter()
                .map(|s| s.name.to_owned())
                .collect::<Vec<_>>(),
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

        assert_eq!(8, parser.order_rules.len());

        let mods = debug_get_mods_from_order_rules(&parser.order_rules);

        match new_unstable_sorter().topo_sort(ESupportedGame::Morrowind, &mods, &parser.order_rules)
        {
            Ok(result) => {
                assert!(
                    check_order(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("Error: {}", e),
        }

        match new_stable_sorter().topo_sort(ESupportedGame::Morrowind, &mods, &parser.order_rules) {
            Ok(result) => {
                assert!(
                    check_order(&result, &parser.order_rules),
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
            assert_eq!(11, parser.warning_rules.len());
        }

        {
            let mut parser = new_tes3_parser();
            parser
                .init_from_file("./tests/plox/rules_note_failing.txt")
                .expect("failed rule parsing");
            assert_eq!(0, parser.warning_rules.len());
        }
    }

    #[test]
    fn test_parse_conflicts() {
        init();

        let mut parser = new_tes3_parser();
        parser
            .init_from_file("./tests/plox/rules_conflict.txt")
            .expect("failed rule parsing");
        assert_eq!(6, parser.warning_rules.len());
    }

    #[test]
    fn test_parse_requires() {
        init();

        let mut parser = new_tes3_parser();
        parser
            .init_from_file("./tests/plox/rules_requires.txt")
            .expect("failed rule parsing");
        assert_eq!(1, parser.warning_rules.len());
    }

    #[test]
    fn test_dump_rules() -> std::io::Result<()> {
        init();

        {
            let mut parser = new_tes3_parser();
            parser.init_from_file("./tests/mlox/mlox_base.txt")?;

            {
                let file = std::fs::File::create("base_rules.json").expect("file create failed");
                serde_json::to_writer_pretty(file, &parser.warning_rules)
                    .expect("serialize failed");
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
                serde_json::to_writer_pretty(file, &parser.warning_rules)
                    .expect("serialize failed");
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
                for rule in parser.warning_rules {
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
                for rule in parser.warning_rules {
                    writeln!(file, "{}", rule).expect("could not write to file");
                }
            }

            Ok(())
        }
    }

    #[test]
    fn test_mlox_user_rules() -> std::io::Result<()> {
        init();

        let mut parser = new_tes3_parser();
        parser.init_from_file("./tests/mlox/mlox_user.txt")?;

        let mut mods = debug_get_mods_from_order_rules(&parser.order_rules);

        let mut rng = thread_rng();
        mods.shuffle(&mut rng);

        // match new_stable_sorter().topo_sort(ESupportedGame::Morrowind, &mods, &parser.order_rules) {
        //     Ok(result) => {
        //         assert!(
        //             check_order(&result, &parser.order_rules),
        //             "stable(true) order is wrong"
        //         );
        //     }
        //     Err(e) => {
        //         panic!("Error: {}", e)
        //     }
        // }

        match new_unstable_sorter().topo_sort(ESupportedGame::Morrowind, &mods, &parser.order_rules)
        {
            Ok(result) => {
                assert!(
                    check_order(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("Error: {}", e),
        }

        Ok(())
    }

    #[test]
    fn test_mlox_base_rules() -> std::io::Result<()> {
        init();

        let mut parser = new_tes3_parser();
        parser.init_from_file("./tests/mlox/mlox_base.txt")?;

        let mut mods = debug_get_mods_from_order_rules(&parser.order_rules);

        let mut rng = thread_rng();
        mods.shuffle(&mut rng);

        match new_stable_sorter().topo_sort(ESupportedGame::Morrowind, &mods, &parser.order_rules) {
            Ok(result) => {
                assert!(
                    check_order(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => {
                panic!("Error: {}", e)
            }
        }

        match new_unstable_sorter().topo_sort(ESupportedGame::Morrowind, &mods, &parser.order_rules)
        {
            Ok(result) => {
                assert!(
                    check_order(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("Error: {}", e),
        }

        Ok(())
    }

    #[allow(dead_code)]
    //#[test]
    fn test_mlox_rules() -> std::io::Result<()> {
        init();

        let mut parser = new_tes3_parser();
        parser.init("./tests/mlox")?;

        let mods = debug_get_mods_from_order_rules(&parser.order_rules);

        // let mut rng = thread_rng();
        // mods.shuffle(&mut rng);

        warn!("MODS: {}", mods.len());

        match new_stable_sorter().topo_sort(ESupportedGame::Morrowind, &mods, &parser.order_rules) {
            Ok(result) => {
                assert!(
                    check_order(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => {
                match new_unstable_sorter().topo_sort(
                    ESupportedGame::Morrowind,
                    &mods,
                    &parser.order_rules,
                ) {
                    Ok(result) => {
                        assert!(
                            check_order(&result, &parser.order_rules),
                            "stable(true) order is wrong"
                        );
                    }
                    Err(e) => panic!("Error: {}", e),
                }
                panic!("Error: {}", e)
            }
        }

        match new_unstable_sorter().topo_sort(ESupportedGame::Morrowind, &mods, &parser.order_rules)
        {
            Ok(result) => {
                assert!(
                    check_order(&result, &parser.order_rules),
                    "stable(true) order is wrong"
                );
            }
            Err(e) => panic!("Error: {}", e),
        }

        Ok(())
    }

    fn new_stable_full_sorter() -> Sorter {
        Sorter::new(sorter::ESortType::StableFull, 1000)
    }

    #[test]
    fn test_optimized_sort() -> std::io::Result<()> {
        init();

        let mut parser = parser::new_tes3_parser();
        parser.init_from_file("./tests/mlox/mlox_base.txt")?;
        let mut mods = debug_get_mods_from_order_rules(&parser.order_rules);

        let mut rng = thread_rng();
        mods.shuffle(&mut rng);
        let mods = mods.into_iter().take(100).collect::<Vec<_>>();

        let full_result = new_stable_full_sorter()
            .topo_sort(ESupportedGame::Morrowind, &mods, &parser.order_rules)
            .expect("rules contain a cycle");
        let opt_result = sorter::new_stable_sorter()
            .topo_sort(ESupportedGame::Morrowind, &mods, &parser.order_rules)
            .expect("opt rules contain a cycle");

        assert_eq!(full_result, opt_result);

        Ok(())
    }

    #[test]
    fn test_optimized_sort_time() -> std::io::Result<()> {
        init();

        let mut parser = parser::new_tes3_parser();
        parser.init_from_file("./tests/mlox/mlox_base.txt")?;
        let mut mods = debug_get_mods_from_order_rules(&parser.order_rules);

        let mut rng = thread_rng();
        let mut times = vec![];
        for n in [64, 128, 256, 512 /* 1024 , 2048 */] {
            mods.shuffle(&mut rng);
            let max = std::cmp::min(n, mods.len() - 1);
            let mods_rnd = mods.clone().into_iter().take(max).collect::<Vec<_>>();

            let now = std::time::Instant::now();
            sorter::new_stable_sorter()
                .topo_sort(ESupportedGame::Morrowind, &mods_rnd, &parser.order_rules)
                .expect("error: ");
            let elapsed = now.elapsed().as_secs();

            times.push((n, elapsed));
        }

        let mut msg = String::new();
        for (n, t) in &times {
            msg += format!("{},{}\n", n, t).as_str();
        }

        // log to file
        // let mut file = File::create("unit_log.txt").expect("could not create log file");
        // file.write_all(msg.as_bytes()).expect("write error");

        // assert
        for (_n, t) in times {
            assert!(t < 4);
        }

        Ok(())
    }

    #[test]
    fn test_gather_mods() {
        init();

        let root_path = "./tests";

        let mods = gather_mods(&root_path, ESupportedGame::Cyberpunk, None);
        assert_eq!(
            mods.iter().map(|s| s.name.to_owned()).collect::<Vec<_>>(),
            vec![
                "a.archive".to_owned(),
                "b.archive".into(),
                "c.archive".into()
            ]
        )
    }

    #[test]
    fn test_parse_header() {
        init();

        {
            let plugin_test_path = PathBuf::from("tests").join("test2.esp");
            let header = parse_header(&plugin_test_path).expect("failed to parse header");

            // check some things
            assert_eq!(
                header.description,
                "The main data file For Morrowind with version 5.3"
            );
            // check master files
            assert!(header.masters.is_none());

            // check version
            let got = get_version(
                plugin_test_path.file_name().unwrap().to_str().unwrap(),
                &Some(header.description),
            );
            let expected = Version::new(5, 3, 0);
            assert_eq!(got.unwrap(), expected);
        }

        {
            let plugin_test_path = PathBuf::from("tests").join("test 1.1.esp");
            let header = parse_header(&plugin_test_path).expect("failed to parse header");

            // check some things
            assert_eq!(
                header.description,
                "The main data file for BloodMoon.\r\n(requires Morrowind.esm to run)"
            );
            // check master files
            assert_eq!(
                header.masters.unwrap(),
                vec![("Morrowind.esm".to_string(), 79837557_u64),]
            );

            // check version
            let got = get_version(
                plugin_test_path.file_name().unwrap().to_str().unwrap(),
                &Some(header.description),
            );
            let expected = Version::new(1, 1, 0);
            assert_eq!(got.unwrap(), expected);
        }
    }
}
