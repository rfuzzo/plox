#[cfg(test)]
mod unit_tests {
    use std::{fs::File, io::Write};

    use rand::{seq::SliceRandom, thread_rng};

    use plox::{
        debug_get_mods_from_rules, expressions::*, get_order_rules, parser, rules::*, sorter,
    };

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
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
            sorter::new_stable_full_sorter()
                .topo_sort(&mods, &order)
                .is_err(),
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
            .collect();

        let result = sorter::new_unstable_sorter()
            .topo_sort(&mods, &order)
            .expect("rules contain a cycle");
        assert!(checkresult(&result, &order), "unstable order is wrong");

        let result = sorter::new_stable_full_sorter()
            .topo_sort(&mods, &order)
            .expect("rules contain a cycle");
        assert!(checkresult(&result, &order), "stable(false) order is wrong");

        let result = sorter::new_stable_sorter()
            .topo_sort(&mods, &order)
            .expect("rules contain a cycle");
        assert!(checkresult(&result, &order), "stable(true) order is wrong");
    }

    #[test]
    fn test_optimized_sort_only() {
        init();

        let rules = parser::new_tes3_parser()
            .parse_rules_from_path("./tests/mlox/mlox_base.txt")
            .expect("rule parse failed");
        let order = get_order_rules(&rules);

        let mut rng = thread_rng();
        let mut mods = debug_get_mods_from_rules(&order);
        mods.shuffle(&mut rng);
        let mods = mods.into_iter().take(100).collect::<Vec<_>>();

        let mut sorter = sorter::new_stable_sorter();
        let result = sorter
            .topo_sort(&mods, &order)
            .expect("opt rules contain a cycle");

        let msg = format!("stable(true) order is wrong");
        assert!(checkresult(&result, &order), "{}", msg);
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

        let full_result = sorter::new_stable_full_sorter()
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
        for n in [64, 128, 256, 512, 1024, 2048] {
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
        let mut file = File::create("unit_log.txt").expect("could not create log file");
        file.write_all(msg.as_bytes()).expect("write error");

        // assert
        for (_n, t) in times {
            assert!(t < 4);
        }
    }

    fn checkresult(result: &[String], order: &Vec<(String, String)>) -> bool {
        let pairs = order;
        for (a, b) in pairs {
            let pos_a = result.iter().position(|x| x == a);
            if pos_a.is_none() {
                continue;
            }
            let pos_b = result.iter().position(|x| x == b);
            if pos_b.is_none() {
                continue;
            }

            if pos_a.unwrap() > pos_b.unwrap() {
                return false;
            }
        }

        true
    }

    #[test]
    fn test_notes() {
        init();

        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        let rules: Vec<_> = [("a", "some a"), ("c", "some b"), ("x", "some x!")]
            .iter()
            .map(|e| Note::new(e.1.into(), &[Atomic::from(e.0).into()]))
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
        init();

        let mods: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        let rules: Vec<Conflict> = vec![
            Conflict::new(
                "some a".into(),
                Atomic::from("a").into(),
                Atomic::from("b").into(),
            ),
            Conflict::new(
                "some b".into(),
                Atomic::from("a").into(),
                Atomic::from("x").into(),
            ),
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
