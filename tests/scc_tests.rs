#[cfg(test)]
mod scc_tests {

    use std::path::PathBuf;

    use log::warn;
    use petgraph::stable_graph::StableGraph;
    use plox::{parser::*, *};
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    #[allow(unused_imports)]
    use rules::{EWarningRule, TWarningRule};

    fn init() {
        let env = env_logger::Env::default()
            .default_filter_or(log_level_to_str(ELogLevel::Debug))
            .default_write_style_or("always");
        let _ = env_logger::Builder::from_env(env).is_test(true).try_init();
    }

    fn graphviz(g: &StableGraph<String, ()>, dir: &std::path::Path) {
        let viz = petgraph::dot::Dot::with_config(&g, &[petgraph::dot::Config::EdgeNoLabel]);
        // write to file

        let file_path = dir.join("graphviz.dot");
        let mut file = std::fs::File::create(file_path).expect("file create failed");
        std::io::Write::write_all(&mut file, format!("{:?}", viz).as_bytes())
            .expect("write failed");
    }

    #[allow(unused_variables)]
    fn clean_mods(plugins: &[PluginData], warning_rules: &[EWarningRule]) -> Vec<PluginData> {
        // lowercase all plugin names
        #[allow(unused_mut)]
        let mut mods_cpy: Vec<_> = plugins
            .iter()
            .map(|f| {
                let mut x = f.clone();
                let name_lc = x.name.to_lowercase();
                x.name = name_lc;
                x
            })
            .collect();

        // let mut warning_rules = warning_rules.to_vec();
        // for rule in warning_rules.iter_mut() {
        //     // only conflict rules
        //     if let EWarningRule::Conflict(ref mut conflict) = rule {
        //         if conflict.eval(&mods_cpy) {
        //             // remove mods
        //             warn!("removing mods: {:?}", conflict.plugins.len());
        //             for mod_name in &conflict.plugins {
        //                 mods_cpy.retain(|x| x.name != *mod_name);
        //             }
        //         }
        //     }
        // }

        mods_cpy
    }

    fn scc(parser: Parser, tmp_dir: PathBuf) -> bool {
        let mut mods = debug_get_mods_from_order_rules(&parser.order_rules);
        mods = clean_mods(&mods, &parser.warning_rules);

        let mut rng = thread_rng();
        mods.shuffle(&mut rng);

        let data = sorter::get_graph_data(&mods, &parser.order_rules, &parser.warning_rules);
        let g = sorter::build_graph(&data);

        graphviz(&g, &tmp_dir);

        // cycle check
        let s = petgraph::algo::toposort(&g, None);
        if let Ok(result) = s {
            // debug print to file
            let mut res = vec![];
            for idx in &result {
                res.push(idx.index());
            }

            let filepath = tmp_dir.join("toposort.json");
            let file = std::fs::File::create(filepath).expect("file create failed");
            serde_json::to_writer_pretty(file, &res).expect("serialize failed");
        } else {
            // tarjan_scc

            let scc = petgraph::algo::tarjan_scc(&g);
            let mut res: Vec<Vec<String>> = vec![];
            for er in &scc {
                if er.len() > 1 {
                    warn!("Found a cycle with {} elements", er.len());
                    let mut cycle = vec![];
                    for e in er {
                        // lookup name
                        let name = data.index_dict_rev[&e.index()].clone();
                        cycle.push(name);
                    }
                    res.push(cycle);
                }
            }
            // debug print to file
            if !res.is_empty() {
                let filepath = tmp_dir.join("tarjan_scc.json");
                let file = std::fs::File::create(filepath).expect("file create failed");
                serde_json::to_writer_pretty(file, &res).expect("serialize failed");

                // find all rules that are part of a cycle
                let mut cycle_rules = vec![];
                for cycle in &res {
                    for rule in &parser.order_rules {
                        // switch
                        let mut names = vec![];
                        if let Some(nearstart) = nearstart2(rule) {
                            names.push(nearstart.names);
                        } else if let Some(nearend) = nearend2(rule) {
                            names.push(nearend.names);
                        } else if let Some(order) = order2(rule.clone()) {
                            names.push(order.names);
                        }

                        // check that the names contain at least 2 mods
                        let mut found = 0;
                        for name in &names {
                            for n in name {
                                if cycle.contains(n) {
                                    found += 1;
                                }
                            }
                        }
                        if found > 1 {
                            cycle_rules.push(rule.clone());
                        }
                    }
                }

                // print cycle rules to file
                let filepath = tmp_dir.join("cycle_rules.json");
                let file = std::fs::File::create(filepath).expect("file create failed");
                serde_json::to_writer_pretty(file, &cycle_rules).expect("serialize failed");
            }

            return false;
        }

        true
    }

    #[test]
    fn scc_user() -> std::io::Result<()> {
        init();

        // delete scc_user folder
        let tmp_dir = PathBuf::from("tmp/scc_user");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        let _ = std::fs::create_dir_all(&tmp_dir);

        let mut parser = new_tes3_parser();
        parser.init_from_file("./tests/mlox/mlox_user.txt")?;

        assert!(scc(parser, tmp_dir));

        Ok(())
    }

    #[test]
    fn scc_base() -> std::io::Result<()> {
        init();

        let tmp_dir = PathBuf::from("tmp/scc_base");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        let _ = std::fs::create_dir_all(&tmp_dir);

        let mut parser = new_tes3_parser();
        parser.init_from_file("./tests/mlox/mlox_base.txt")?;

        assert!(scc(parser, tmp_dir));

        Ok(())
    }

    #[test]
    fn scc_full() -> std::io::Result<()> {
        init();

        // delete scc_full folder
        let tmp_dir = PathBuf::from("tmp/scc_full");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        let _ = std::fs::create_dir_all(&tmp_dir);

        let mut parser = new_tes3_parser();
        parser.parse("./tests/mlox")?;

        assert!(scc(parser, tmp_dir));

        Ok(())
    }
}
