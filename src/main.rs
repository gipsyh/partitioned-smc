mod automata;
mod bdd;

use automata::BuchiAutomata;
use cudd::{Cudd, DdNode};
use smv::{bdd::SmvBdd, Expr, Prefix, Smv};
use std::{process::Command, time::Instant};

struct PartitionedSmc {
    cudd: Cudd,
    trans: DdNode,
    init: DdNode,
    automata: BuchiAutomata,
}

impl PartitionedSmc {
    fn new(cudd: Cudd, trans: DdNode, init: DdNode, automata: BuchiAutomata) -> Self {
        Self {
            cudd,
            trans,
            init,
            automata,
        }
    }

    pub fn pre_image(&mut self, bdd: &DdNode) -> DdNode {
        let num_var = self.cudd.num_var() / 2;
        let bdd = self.cudd.swap_vars(&bdd, 0..num_var, num_var..2 * num_var);
        let bdd = bdd & &self.trans;
        self.cudd.exist_abstract(&bdd, num_var..2 * num_var)
    }

    pub fn post_image(&mut self, bdd: &DdNode) -> DdNode {
        let num_var = self.cudd.num_var() / 2;
        let bdd = bdd & &self.trans;
        let bdd = self.cudd.exist_abstract(&bdd, 0..num_var);
        self.cudd.swap_vars(&bdd, num_var..2 * num_var, 0..num_var)
    }

    fn reachable_state(&mut self, from: &[DdNode], forward: bool) -> Vec<DdNode> {
        assert!(from.len() == self.automata.num_state());
        let mut frontier = from.to_vec();
        let mut reach = vec![self.cudd.constant(false); self.automata.num_state()];
        loop {
            let mut new_frontier = vec![self.cudd.constant(false); self.automata.num_state()];
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
            if new_frontier.iter().all(|bdd| bdd.is_constant(false)) {
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
        let mut fair_states = vec![self.cudd.constant(false); self.automata.num_state()];
        for state in self.automata.accepting_states.iter() {
            fair_states[*state] = self.cudd.constant(true);
        }
        loop {
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
        fair_states
    }

    fn check(&mut self) -> bool {
        let fair_states = self.fair_states();
        let mut reach = vec![self.cudd.constant(false); self.automata.num_state()];
        for init_state in self.automata.init_states.iter() {
            for (next, label) in self.automata.forward[*init_state].iter() {
                reach[*next] = &reach[*next] | &(label & &self.init);
            }
        }
        let forward = self.reachable_state(&reach, true);
        for i in 0..forward.len() {
            reach[i] = &forward[i] | &reach[i];
        }
        for accept in self.automata.accepting_states.iter() {
            if &reach[*accept] & &fair_states[*accept] != self.cudd.constant(false) {
                return false;
            }
        }
        return true;
    }
}

fn main() {
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/mutex/mutex-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/short/short-flat.smv").unwrap();
    let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/ring/ring-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/examples/counter/10bit/counter-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/abp/abp8-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/example_cmu/dme1-flat.smv").unwrap();
    let smv_bdd = SmvBdd::new(&smv);
    dbg!(&smv_bdd.trans);
    dbg!(&smv_bdd.symbols);
    dbg!(&smv_bdd.init);
    let mut fairness = Expr::LitExpr(true);
    for fair in smv.fairness.iter() {
        let fair = Expr::PrefixExpr(
            Prefix::LtlGlobally,
            Box::new(Expr::PrefixExpr(Prefix::LtlFinally, Box::new(fair.clone()))),
        );
        fairness = fairness & fair;
    }
    let mut cudd = smv_bdd.cudd.clone();
    for ltl in &smv.ltlspecs[..] {
        println!("'{}'", !ltl.clone() & fairness.clone());
        let ltl2dfa = Command::new("/root/ltl2ba-1.3/ltl2ba")
            .arg("-f")
            .arg(format!("{}", !ltl.clone() & fairness.clone()))
            .output()
            .unwrap();
        let ba = String::from_utf8_lossy(&ltl2dfa.stdout);
        let ba = BuchiAutomata::parse(ba.as_ref(), &mut cudd, &smv_bdd.symbols);
        let mut partitioned_smc = PartitionedSmc::new(
            cudd.clone(),
            smv_bdd.trans.clone(),
            smv_bdd.init.clone(),
            ba,
        );
        let start = Instant::now();
        dbg!(partitioned_smc.check());
        println!("{:?}", start.elapsed());
    }
}
