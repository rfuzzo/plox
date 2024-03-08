#[cfg(test)]
mod unit_tests {

    use rand::{seq::SliceRandom, thread_rng};

    use plox::{
        rules::{EOrderRule, Order},
        sorter::{self, Sorter},
        wild_contains, *,
    };

    fn init() {
        let env = env_logger::Env::default()
            .default_filter_or(log_level_to_str(ELogLevel::Debug))
            .default_write_style_or("always");
        let _ = env_logger::Builder::from_env(env).is_test(true).try_init();
    }

    fn new_stable_full_sorter() -> Sorter {
        Sorter::new(sorter::ESortType::StableFull, 1000)
    }

    #[test]
    fn test_cycle() {
        init();

        let order = [
            Order::from("a", "b").into(),
            Order::from("b", "c").into(),
            Order::from("d", "e").into(),
            Order::from("b", "a").into(),
        ];

        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        assert!(
            sorter::new_unstable_sorter()
                .topo_sort(&mods, &order)
                .is_err(),
            "unstable rules do not contain a cycle"
        );

        assert!(
            new_stable_full_sorter().topo_sort(&mods, &order).is_err(),
            "stable(false) rules do not contain a cycle"
        );

        assert!(
            sorter::new_stable_sorter()
                .topo_sort(&mods, &order)
                .is_err(),
            "stable(true) rules do not contain a cycle"
        );
    }

    #[test]
    fn test_ordering() {
        init();

        let order = [
            Order::from("b", "a").into(),
            Order::from("b", "c").into(),
            Order::from("d", "e").into(),
            Order::from("e", "c").into(),
            Order::from("test.archive", "test2.archive").into(),
        ];

        let mods = ["d", "e", "f", "g", "a", "b", "c"]
            .iter()
            .map(|e| (*e).into())
            .collect::<Vec<_>>();

        match sorter::new_unstable_sorter().topo_sort(&mods, &order) {
            Ok(result) => {
                assert!(checkresult(&result, &order), "stable(true) order is wrong");
            }
            Err(e) => panic!("Error: {}", e),
        }

        match new_stable_full_sorter().topo_sort(&mods, &order) {
            Ok(result) => {
                assert!(checkresult(&result, &order), "stable(true) order is wrong");
            }
            Err(e) => panic!("Error: {}", e),
        }

        match sorter::new_stable_sorter().topo_sort(&mods, &order) {
            Ok(result) => {
                assert!(checkresult(&result, &order), "stable(true) order is wrong");
            }
            Err(e) => panic!("Error: {}", e),
        }
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
            .topo_sort(&mods, &parser.order_rules)
            .expect("rules contain a cycle");
        let opt_result = sorter::new_stable_sorter()
            .topo_sort(&mods, &parser.order_rules)
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
                .topo_sort(&mods_rnd, &parser.order_rules)
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

    fn checkresult(result: &[String], order_rules: &[EOrderRule]) -> bool {
        let order = plox::get_ordering_from_order_rules(order_rules);
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
