use std::collections::HashMap;

use log::warn;
use petgraph::{graph::NodeIndex, stable_graph::StableGraph};

use crate::{
    get_ordering_from_order_rules, nearend2, nearstart2, wild_contains, EOrderRule, ESupportedGame,
    EWarningRule, PluginData,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ESortType {
    Unstable,
    StableOpt,
    StableFull,
}

pub fn new_unstable_sorter() -> Sorter {
    Sorter::new(ESortType::Unstable, 0)
}

pub fn new_stable_sorter() -> Sorter {
    Sorter::new(ESortType::StableOpt, 100)
}

pub struct GraphData {
    pub index_dict: HashMap<String, usize>,
    pub index_dict_rev: HashMap<usize, String>,
    pub edges: Vec<(usize, usize)>,
}

pub struct Sorter {
    pub sort_type: ESortType,
    pub max_iterations: usize,
}

impl Sorter {
    pub fn new(sort_type: ESortType, max_iterations: usize) -> Self {
        Self {
            sort_type,
            max_iterations,
        }
    }

    /// Sorts the input mods topologically. Mods input is case sensitive!
    ///
    /// # Panics
    ///
    /// Panics if .
    ///
    /// # Errors
    ///
    /// This function will return an error if any parsing fails
    pub fn topo_sort(
        &mut self,
        game: ESupportedGame,
        plugins: &[PluginData],
        order_rules: &[EOrderRule],
        warn_rules: &[EWarningRule],
    ) -> Result<Vec<String>, &'static str> {
        // early out
        if order_rules.is_empty() {
            log::info!("No order rules found, nothing to sort");
            return Err("No order rules found");
        }

        let data = get_graph_data(plugins, order_rules, warn_rules);
        let g = build_graph(&data);

        let GraphData {
            index_dict,
            index_dict_rev,
            mut edges,
            ..
        } = data;

        // cycle check
        if self.sort_type == ESortType::Unstable {
            let s = petgraph::algo::toposort(&g, None);
            let sort;
            if let Ok(result) = s {
                sort = result;
            } else {
                return Err("Graph contains a cycle");
            }

            // map sorted index back to mods
            let mut result = vec![];
            for idx in sort {
                let plugin = &plugins[idx.index()];
                result.push(plugin.name.to_owned());
            }
            return Ok(result);
        }

        // sort
        let mut mods = plugins
            .iter()
            .map(|f| f.name.to_lowercase())
            .collect::<Vec<String>>();

        // nearstart rules
        for nearstart in order_rules
            .iter()
            .filter_map(nearstart2)
            .flat_map(|f| f.names)
            .rev()
        {
            if let Some(results) = wild_contains(&mods, &nearstart) {
                // push to start of mods
                for r in results {
                    let index = mods.iter().position(|f| f == &r).unwrap();
                    let element = mods.remove(index);
                    mods.insert(0, element);
                }
            }
        }

        // nearend rules
        for nearend in order_rules
            .iter()
            .filter_map(nearend2)
            .flat_map(|f| f.names)
            .rev()
        {
            if let Some(results) = wild_contains(&mods, &nearend) {
                // push to end of mods
                for r in results {
                    let index = mods.iter().position(|f| f == &r).unwrap();
                    let element = mods.remove(index);
                    mods.push(element);
                }
            }
        }

        let n = plugins.len();

        let mut index = 0;

        edges.sort_by_key(|k| k.0);

        for i in 1..self.max_iterations {
            let any_change = self.stable_topo_sort_inner(
                n,
                &edges,
                &index_dict,
                &index_dict_rev,
                &mut mods,
                &mut index,
            );

            // sort again
            if !any_change {
                // sort esms now?
                if game == ESupportedGame::Morrowind || game == ESupportedGame::Openmw {
                    // put all items in mods_copy ending with .esm at the start
                    let mut esms = vec![];
                    for (i, m) in mods.iter().enumerate() {
                        if m.ends_with(".esm") || m.ends_with(".omwgame") {
                            esms.push(i);
                        }
                    }
                    // now sort the mods_copy list
                    for (last_i, i) in esms.iter().enumerate() {
                        let element = mods.remove(*i);
                        mods.insert(last_i, element);
                    }

                    // put standard tes3 esms at the start
                    // if mods_copy.contains(&"bloodmoon.esm".into()) {
                    //     let index = mods_copy.iter().position(|f| f == "bloodmoon.esm").unwrap();
                    //     let element = mods_copy.remove(index);
                    //     mods_copy.insert(0, element);
                    // }

                    // if mods_copy.contains(&"tribunal.esm".into()) {
                    //     let index = mods_copy.iter().position(|f| f == "tribunal.esm").unwrap();
                    //     let element = mods_copy.remove(index);
                    //     mods_copy.insert(0, element);
                    // }

                    if mods.contains(&"morrowind.esm".into()) {
                        let index = mods.iter().position(|f| f == "morrowind.esm").unwrap();
                        let element = mods.remove(index);
                        mods.insert(0, element);
                    }
                }

                // Return the sorted vector
                // map sorted index back to mods
                let mut result = vec![];
                for lower_case_name in mods {
                    let idx = index_dict[&lower_case_name.clone()];
                    let plugin = &plugins[idx];
                    result.push(plugin.name.to_owned());
                }
                return Ok(result);
            }

            if let Some(edge) = edges.get(index) {
                let resolved_0 = &index_dict_rev[&edge.0];
                let resolved_1 = &index_dict_rev[&edge.1];
                log::debug!("{}, index {} ({}, {})", i, index, resolved_0, resolved_1);
            } else {
                log::debug!("{}, index {}", i, index);
            }
        }

        log::error!("Out of iterations");
        Err("Out of iterations")
    }

    pub fn stable_topo_sort_inner(
        &self,
        n: usize,
        edges: &[(usize, usize)],
        index_dict: &HashMap<String, usize>,
        index_dict_rev: &HashMap<usize, String>,
        result: &mut Vec<String>,
        last_index: &mut usize,
    ) -> bool {
        match self.sort_type {
            ESortType::Unstable => panic!("not supported"),
            ESortType::StableOpt => {
                Self::stable_topo_sort_opt2(n, edges, index_dict_rev, result, last_index)
            }
            ESortType::StableFull => {
                Self::stable_topo_sort_full(n, edges, index_dict, result, last_index)
            }
        }
    }

    pub fn stable_topo_sort_full(
        n: usize,
        edges: &[(usize, usize)],
        index_dict: &HashMap<String, usize>,
        result: &mut Vec<String>,
        last_index: &mut usize,
    ) -> bool {
        for i in 0..n {
            for j in 0..i {
                let x = index_dict[result[i].as_str()];
                let y = index_dict[result[j].as_str()];
                if edges.contains(&(x, y)) {
                    let t = result[i].to_owned();
                    result.remove(i);
                    result.insert(j, t);

                    *last_index = j;

                    return true;
                }
            }
        }
        false
    }

    pub fn stable_topo_sort_opt2(
        _n: usize,
        edges: &[(usize, usize)],
        index_dict_rev: &HashMap<usize, String>,
        result: &mut Vec<String>,
        last_index: &mut usize,
    ) -> bool {
        // optimize B: only check edges
        let mut b = false;
        for (idx, edge) in edges.iter().enumerate() {
            let i = edge.0;
            let j = edge.1;

            let x = &index_dict_rev[&i];
            let y = &index_dict_rev[&j];

            let idx_of_x = result.iter().position(|f| f == x).unwrap();
            let idx_of_y = result.iter().position(|f| f == y).unwrap();

            // if i not before j x should be before y
            if idx_of_x > idx_of_y {
                let t = result[idx_of_x].to_owned();
                result.remove(idx_of_x);
                result.insert(idx_of_y, t);

                *last_index = idx;

                b = true;

                // logging
                //log::debug!("\t{}: {} -> {}: {}", idx_of_x, x, idx_of_y, y);
            }
        }

        b
    }
}

pub fn get_graph_data(
    plugins: &[PluginData],
    order_rules: &[EOrderRule],
    _warn_rules: &[EWarningRule],
) -> GraphData {
    // build hashmaps for lookup
    let mut index_dict: HashMap<String, usize> = HashMap::new();
    let mut index_dict_rev: HashMap<usize, String> = HashMap::default();
    let mut plugin_map: HashMap<usize, PluginData> = HashMap::default();

    for (i, plugin_data) in plugins.iter().enumerate() {
        let lower_case = plugin_data.name.to_lowercase();

        index_dict.insert(lower_case.clone(), i);
        index_dict_rev.insert(i, lower_case.clone());

        plugin_map.insert(i, plugin_data.to_owned());
    }

    // add edges from order rules
    let mods = plugins
        .iter()
        .map(|f| f.name.to_lowercase())
        .collect::<Vec<String>>();

    let order_pairs = get_ordering_from_order_rules(order_rules);
    let mut edges: Vec<(usize, usize)> = vec![];
    for (a, b) in order_pairs {
        if let Some(results_for_a) = wild_contains(&mods, &a) {
            if let Some(results_for_b) = wild_contains(&mods, &b) {
                // foreach esp i, add an edge to all esps j
                for i in &results_for_a {
                    for j in &results_for_b {
                        if i == j {
                            warn!("Skipping circular edge: {}", i);
                            continue;
                        }
                        let idx_a = index_dict[i.as_str()];
                        let idx_b = index_dict[j.as_str()];

                        if !edges.contains(&(idx_a, idx_b)) {
                            edges.push((idx_a, idx_b));
                        }
                    }
                }
            }
        }
    }

    // add edges from masters
    for mod_data in plugins.iter() {
        // add an edge from the mod to all its masters
        let idx = index_dict[&mod_data.name.to_lowercase()];
        if let Some(masters) = &mod_data.masters {
            for (master, _hash) in masters {
                let master = master.to_lowercase();
                if let Some(results) = wild_contains(&mods, &master) {
                    for result in results {
                        let idx_master = index_dict[&result];
                        let edge = (idx_master, idx);
                        if !edges.contains(&edge) {
                            edges.push(edge);
                        }
                    }
                }
            }
        }
    }

    // return
    GraphData {
        index_dict,
        index_dict_rev,
        edges,
    }
}

pub fn build_graph(data: &GraphData) -> StableGraph<String, ()> {
    let GraphData {
        index_dict_rev,
        edges,
        ..
    } = data;

    // create graph from edges
    let mut g = StableGraph::<String, ()>::with_capacity(index_dict_rev.len(), edges.len());
    for n in 0..index_dict_rev.len() {
        let name = index_dict_rev[&n].clone();
        g.add_node(name);
    }
    // add edges
    for edge in edges.iter() {
        g.add_edge(NodeIndex::new(edge.0), NodeIndex::new(edge.1), ());
    }

    g
}
