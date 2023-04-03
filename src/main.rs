mod automata;
mod bdd;

use crate::{automata::counter, bdd::get_trans_bdd};
use aig::Aig;
use automata::BuchiAutomata;
use cudd::{Cudd, DdNode};

struct PartitionedSmc {
    aig: Aig,
    cudd: Cudd,
    trans_bdd: DdNode,
    automata: BuchiAutomata,
}

impl PartitionedSmc {
    fn new(aig: Aig) -> Self {
        let mut cudd = Cudd::new();
        let trans_bdd = get_trans_bdd(&aig, &mut cudd);
        let automata = counter(&mut cudd);
        Self {
            aig,
            cudd,
            trans_bdd,
            automata,
        }
    }

    pub fn pre_image(&mut self, bdd: &DdNode) -> DdNode {
        let num_var = self.cudd.num_var() / 2;
        let bdd = self.cudd.swap_vars(&bdd, 0..num_var, num_var..2 * num_var);
        let bdd = &bdd & &self.trans_bdd;
        self.cudd.exist_abstract(&bdd, num_var..2 * num_var)
    }

    pub fn post_image(&mut self, bdd: &DdNode) -> DdNode {
        let num_var = self.cudd.num_var() / 2;
        let bdd = bdd & &self.trans_bdd;
        let bdd = self.cudd.exist_abstract(&bdd, 0..num_var);
        self.cudd.swap_vars(&bdd, num_var..2 * num_var, 0..num_var)
    }

    fn reachable_state(&mut self, from: &[DdNode], forward: bool) -> Vec<DdNode> {
        assert!(from.len() == self.automata.num_state());
        let mut frontier = from.to_vec();
        let mut reach = vec![self.cudd.false_node(); self.automata.num_state()];
        loop {
            // dbg!(&reach);
            let mut new_frontier = vec![self.cudd.false_node(); self.automata.num_state()];
            for i in 0..frontier.len() {
                let (image, trans) = if forward {
                    (
                        self.post_image(&frontier[i]),
                        self.automata.forward[i].iter(),
                    )
                } else {
                    (
                        self.pre_image(&frontier[i]),
                        self.automata.backward[i].iter(),
                    )
                };
                for (next, label) in trans {
                    let update = &image & label & !&reach[*next];
                    new_frontier[*next] = &new_frontier[*next] | &update;
                }
            }
            if new_frontier.iter().all(|bdd| bdd.is_false()) {
                break;
            }
            for i in 0..new_frontier.len() {
                reach[i] = &reach[i] | &new_frontier[i];
                frontier[i] = new_frontier[i].clone();
            }
        }
        reach
    }

    fn fair_states(&mut self) -> Vec<DdNode> {
        let mut fair_states = vec![self.cudd.false_node(); self.automata.num_state()];
        for state in self.automata.accepting_states.iter() {
            fair_states[*state] = self.cudd.true_node();
        }
        loop {
            dbg!(&fair_states);
            let backward = self.reachable_state(&fair_states, false);
            let mut new_fair_sets = Vec::new();
            for i in 0..fair_states.len() {
                new_fair_sets.push(&fair_states[i] & &backward[i]);
            }
            if fair_states == new_fair_sets {
                break;
            }
            fair_states = new_fair_sets;
        }
        dbg!(&fair_states);
        fair_states
    }

    fn check(&mut self) {
        self.fair_states();
        let init_cube: Vec<DdNode> = self
            .aig
            .latchs
            .iter()
            .map(|l| !self.cudd.ith_var(l.input))
            .collect();
        let init_bdd = self.cudd.cube_bdd(init_cube.iter());
        let mut reach = vec![self.cudd.false_node(); self.automata.num_state()];
        for init_state in self.automata.init_states.iter() {
            for (next, label) in self.automata.forward[*init_state].iter() {
                reach[*next] = &reach[*next] | &(label & &init_bdd);
            }
        }
        let mut forward = self.reachable_state(&reach, true);
        for i in 0..forward.len() {
            forward[i] = &forward[i] | &reach[i];
        }
        dbg!(reach[2].is_false());
    }
}

fn main() {
    let aig = Aig::from_file("../test/counter.aag").unwrap();
    let mut partitioned_smc = PartitionedSmc::new(aig);
    partitioned_smc.check();
}
