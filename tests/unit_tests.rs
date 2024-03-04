#[cfg(test)]
mod unit_tests {

    use rand::{seq::SliceRandom, thread_rng};

    use plox::{
        debug_get_mods_from_rules, get_order_rules, parser,
        sorter::{self, Sorter},
        wild_contains,
    };

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn new_stable_full_sorter() -> Sorter {
        Sorter::new(sorter::ESortType::StableFull, 1000)
    }

    #[test]
    fn test_cycle() {
        init();

        let order = [("a", "b"), ("b", "c"), ("d", "e"), ("b", "a")]
            .iter()
            .map(|e| (e.0.to_owned(), e.1.to_owned()))
            .collect();

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
            ("b", "a"),
            ("b", "c"),
            ("d", "e"),
            ("e", "c"),
            ("test.archive", "test2.archive"),
        ]
        .iter()
        .map(|e| (e.0.to_owned(), e.1.to_owned()))
        .collect();

        let mods = ["d", "e", "f", "g", "a", "b", "c"]
            .iter()
            .map(|e| (*e).into())
            .collect::<Vec<_>>();

        let result = sorter::new_unstable_sorter()
            .topo_sort(&mods, &order)
            .expect("rules contain a cycle");
        assert!(checkresult(&result, &order), "unstable order is wrong");

        let result = new_stable_full_sorter()
            .topo_sort(&mods, &order)
            .expect("rules contain a cycle");
        assert!(checkresult(&result, &order), "stable(false) order is wrong");

        let result = sorter::new_stable_sorter()
            .topo_sort(&mods, &order)
            .expect("rules contain a cycle");
        assert!(checkresult(&result, &order), "stable(true) order is wrong");
    }

    #[test]
    fn test_optimized_sort() {
        init();

        let rules = parser::new_tes3_parser()
            .parse_rules_from_path("./tests/mlox/mlox_base.txt")
            .expect("rule parse failed");
        let order = get_order_rules(&rules);

        let mut rng = thread_rng();
        let mut mods = debug_get_mods_from_rules(&order);
        mods.shuffle(&mut rng);
        let mods = mods.into_iter().take(100).collect::<Vec<_>>();

        let full_result = new_stable_full_sorter()
            .topo_sort(&mods, &order)
            .expect("rules contain a cycle");
        let opt_result = sorter::new_stable_sorter()
            .topo_sort(&mods, &order)
            .expect("opt rules contain a cycle");

        assert_eq!(full_result, opt_result);
    }

    #[test]
    fn test_optimized_sort_time() {
        init();

        let rules = parser::new_tes3_parser()
            .parse_rules_from_path("./tests/mlox/mlox_base.txt")
            .expect("rule parse failed");
        let order = get_order_rules(&rules);

        let mut rng = thread_rng();
        let mut mods = debug_get_mods_from_rules(&order);
        let mut times = vec![];
        for n in [64, 128, 256, 512, 1024, 2048, 4096] {
            mods.shuffle(&mut rng);
            let max = std::cmp::min(n, mods.len() - 1);
            let mods_rnd = mods.clone().into_iter().take(max).collect::<Vec<_>>();

            let now = std::time::Instant::now();
            sorter::new_stable_sorter()
                .topo_sort(&mods_rnd, &order)
                .expect("opt rules contain a cycle");
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
