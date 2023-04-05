mod automata;
mod bdd;

use automata::BuchiAutomata;
use cudd::{Cudd, DdNode};
use smv::{bdd::SmvBdd, Expr, Prefix, Smv};
use std::{
    process::Command,
    time::{Duration, Instant},
};

struct PartitionedSmc {
    cudd: Cudd,
    trans: DdNode,
    init: DdNode,
    automata: BuchiAutomata,
    and_time: Duration,
    image_time: Duration,
}

impl PartitionedSmc {
    fn new(cudd: Cudd, trans: DdNode, init: DdNode, automata: BuchiAutomata) -> Self {
        Self {
            cudd,
            trans,
            init,
            automata,
            and_time: Duration::default(),
            image_time: Duration::default(),
        }
    }

    pub fn pre_image(&mut self, bdd: &DdNode) -> DdNode {
        let num_var = self.cudd.num_var() / 2;
        let bdd = self.cudd.swap_vars(
            bdd,
            (0..num_var).map(|x| x * 2),
            (0..num_var).map(|x| x * 2 + 1),
        );
        let bdd = bdd & &self.trans;
        self.cudd
            .exist_abstract(&bdd, (0..num_var).map(|x| x * 2 + 1))
    }

    pub fn post_image(&mut self, bdd: &DdNode) -> DdNode {
        let num_var = self.cudd.num_var() / 2;
        let bdd = bdd & &self.trans;
        let bdd = self.cudd.exist_abstract(&bdd, (0..num_var).map(|x| x * 2));
        self.cudd.swap_vars(
            &bdd,
            (0..num_var).map(|x| x * 2 + 1),
            (0..num_var).map(|x| x * 2),
        )
    }

    fn reachable_state_image_first(
        &mut self,
        from: &[DdNode],
        forward: bool,
        constraint: Option<&[DdNode]>,
    ) -> Vec<DdNode> {
        assert!(from.len() == self.automata.num_state());
        let mut automata_trans = if forward {
            self.automata.forward.clone()
        } else {
            self.automata.backward.clone()
        };
        let mut frontier = from.to_vec();
        let mut reach = vec![self.cudd.constant(false); self.automata.num_state()];
        let mut y = 0;
        loop {
            y += 1;
            dbg!(y);
            let mut new_frontier = vec![self.cudd.constant(false); self.automata.num_state()];
            for i in 0..frontier.len() {
                let image = if forward {
                    self.post_image(&frontier[i])
                } else {
                    self.pre_image(&frontier[i])
                };
                for (next, label) in automata_trans[i].iter_mut() {
                    *label = label.as_ref() & !&reach[*next];
                    if let Some(constraint) = constraint {
                        *label = label.as_ref() & &constraint[*next];
                    }
                    let update = &image & &label;
                    new_frontier[*next] = &new_frontier[*next] | &update;
                    reach[*next] = &reach[*next] | update;
                }
            }
            if new_frontier.iter().all(|bdd| bdd.is_constant(false)) {
                break;
            }
            frontier = new_frontier;
        }
        reach
    }

    fn reachable_state_propagate_first(&mut self, from: &[DdNode], forward: bool) -> Vec<DdNode> {
        assert!(from.len() == self.automata.num_state());
        let automata_trans = if forward {
            self.automata.forward.clone()
        } else {
            self.automata.backward.clone()
        };
        let mut frontier = from.to_vec();
        let mut reach = vec![self.cudd.constant(false); self.automata.num_state()];
        let mut y = 0;
        loop {
            y += 1;
            dbg!(y);
            let mut new_frontier = vec![self.cudd.constant(false); self.automata.num_state()];
            let mut tmp = vec![self.cudd.constant(false); self.automata.num_state()];
            for i in 0..frontier.len() {
                for (next, label) in automata_trans[i].iter() {
                    let update = &frontier[i] & label;
                    tmp[*next] |= update;
                }
            }
            for i in 0..tmp.len() {
                let update = if forward {
                    self.post_image(&tmp[i])
                } else {
                    self.pre_image(&tmp[i])
                } & !&reach[i];
                reach[i] |= &update;
                new_frontier[i] |= update;
            }
            if new_frontier.iter().all(|bdd| bdd.is_constant(false)) {
                break;
            }
            frontier = new_frontier;
        }
        reach
    }

    fn fair_states(&mut self) -> Vec<DdNode> {
        let mut fair_states = vec![self.cudd.constant(false); self.automata.num_state()];
        for state in self.automata.accepting_states.iter() {
            fair_states[*state] = self.cudd.constant(true);
        }
        let candidate = self.reachable_state_propagate_first(&fair_states, true);
        let mut x = 0;
        loop {
            x += 1;
            dbg!(x);
            let backward = self.reachable_state_image_first(&fair_states, false, Some(&candidate));
            // let backward = self.reachable_state_image_first(&fair_states, false, None);
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
        // dbg!(&fair_states);
        dbg!("xxx");
        let mut reach = vec![self.cudd.constant(false); self.automata.num_state()];
        for init_state in self.automata.init_states.iter() {
            reach[*init_state] |= &self.init;
        }
        let forward = self.reachable_state_propagate_first(&reach, true);
        for i in 0..forward.len() {
            reach[i] = &forward[i] | &reach[i];
        }
        for accept in self.automata.accepting_states.iter() {
            if &reach[*accept] & &fair_states[*accept] != self.cudd.constant(false) {
                return false;
            }
        }
        true
    }
}

fn main() {
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/mutex/mutex-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/short/short-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/ring/ring-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/examples/counter/10bit/counter-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/abp/abp8-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/abp/abp4-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/dme/dme3-flat.smv").unwrap();
    let smv =
        Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/example_cmu/dme1-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/prod-cons/prod-cons-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/production-cell/production-cell-flat.smv")
    // .unwrap();
    let smv_bdd = SmvBdd::new(&smv);
    dbg!(&smv_bdd.cudd);
    // dbg!(&smv_bdd.trans);
    // dbg!(&smv_bdd.symbols);
    // dbg!(&smv_bdd.init);
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
        dbg!(smv_bdd.symbols.len());
        dbg!(ba.num_state());
        let mut partitioned_smc = PartitionedSmc::new(
            cudd.clone(),
            smv_bdd.trans.clone(),
            smv_bdd.init.clone(),
            ba,
        );
        let start = Instant::now();
        dbg!(partitioned_smc.check());
        dbg!(partitioned_smc.and_time);
        dbg!(partitioned_smc.image_time);
        println!("{:?}", start.elapsed());
    }
}
