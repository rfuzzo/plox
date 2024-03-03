use std::collections::HashMap;

use log::{error, info};
use toposort_scc::IndexGraph;

use crate::wild_contains;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ESortType {
    Unstable,
    StableOpt,
    StableFull,
}

pub fn new_unstable_sorter() -> Sorter {
    Sorter::new(ESortType::Unstable)
}

pub fn new_stable_sorter() -> Sorter {
    Sorter::new(ESortType::StableOpt)
}

pub fn new_stable_full_sorter() -> Sorter {
    Sorter::new(ESortType::StableFull)
}

pub struct Sorter {
    pub sort_type: ESortType,
}

impl Sorter {
    pub fn new(sort_type: ESortType) -> Self {
        Self { sort_type }
    }

    pub fn stable_topo_sort_inner(
        &self,
        n: usize,
        edges: &[(usize, usize)],
        index_dict: &HashMap<&str, usize>,
        index_dict_rev: &HashMap<usize, &str>,
        result: &mut Vec<String>,
        last_index: &mut usize,
    ) -> bool {
        match self.sort_type {
            ESortType::Unstable => panic!("not supported"),
            ESortType::StableOpt => {
                Self::stable_topo_sort_opt(n, edges, index_dict, index_dict_rev, result, last_index)
            }
            ESortType::StableFull => {
                Self::stable_topo_sort_full(n, edges, index_dict, result, last_index)
            }
        }
    }

    pub fn stable_topo_sort_full(
        n: usize,
        edges: &[(usize, usize)],
        index_dict: &HashMap<&str, usize>,
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

    pub fn stable_topo_sort_opt(
        _n: usize,
        edges: &[(usize, usize)],
        _index_dict: &HashMap<&str, usize>,
        index_dict_rev: &HashMap<usize, &str>,
        result: &mut Vec<String>,
        last_index: &mut usize,
    ) -> bool {
        // optimize B: only check edges
        for (idx, edge) in edges.iter().enumerate() {
            let i = edge.0;
            let j = edge.1;

            let x = index_dict_rev[&i];
            let y = index_dict_rev[&j];

            let idx_of_x = result.iter().position(|f| f == x).unwrap();
            let idx_of_y = result.iter().position(|f| f == y).unwrap();

            // if i not before j x should be before y
            if idx_of_x > idx_of_y {
                let t = result[idx_of_x].to_owned();
                result.remove(idx_of_x);
                result.insert(idx_of_y, t);

                *last_index = idx;

                return true;
            }
        }

        false
    }

    pub fn topo_sort(
        &mut self,
        mods: &Vec<String>,
        order: &Vec<(String, String)>,
    ) -> Result<Vec<String>, &'static str> {
        let mut g = IndexGraph::with_vertices(mods.len());

        let mut index_dict: HashMap<&str, usize> = HashMap::new();
        for (i, m) in mods.iter().enumerate() {
            index_dict.insert(m, i);
        }

        // add edges
        let mut edges: Vec<(usize, usize)> = vec![];
        for (a, b) in order {
            if let Some(results_for_a) = wild_contains(mods, a) {
                if let Some(results_for_b) = wild_contains(mods, b) {
                    // e.g. all esms before all esps
                    // [ORDER]
                    // *.esm
                    // *.esp
                    // forach esm i, add an edge to all esps j
                    for i in &results_for_a {
                        for j in &results_for_b {
                            let idx_a = index_dict[i.as_str()];
                            let idx_b = index_dict[j.as_str()];

                            if !edges.contains(&(idx_a, idx_b)) {
                                edges.push((idx_a, idx_b));
                                g.add_edge(idx_a, idx_b);
                            }
                        }
                    }
                }
            }
        }

        // cycle check
        let sort;
        if let Some(result) = g.clone().toposort() {
            sort = result;
        } else {
            error!("Graph contains a cycle");
            let err = g.scc();
            error!("SCC: {}", err.len());
            for er in err {
                error!("cycles:");
                error!(
                    "{}",
                    er.iter()
                        .map(|f| f.to_string())
                        .collect::<Vec<_>>()
                        .join(";")
                );
            }

            return Err("Graph contains a cycle");
        }

        if self.sort_type == ESortType::Unstable {
            let r = sort.iter().map(|f| mods[*f].to_owned()).collect::<Vec<_>>();
            return Ok(r);
        }

        // sort
        let mut result: Vec<String> = mods.iter().map(|e| (*e).to_owned()).collect();
        info!("{result:?}");

        // reverse
        let mut index_dict_rev: HashMap<usize, &str> = HashMap::default();
        for (k, v) in &index_dict {
            index_dict_rev.insert(*v, k);
        }

        let mut index = 0;
        let max_loop = 10000;

        for _n in 1..max_loop {
            if !self.stable_topo_sort_inner(
                mods.len(),
                &edges,
                &index_dict,
                &index_dict_rev,
                &mut result,
                &mut index,
            ) {
                // Return the sorted vector
                return Ok(result);
            }
        }

        Err("Out of iterations")
    }
}
