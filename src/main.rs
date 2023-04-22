#![feature(stmt_expr_attributes)]

mod automata;
mod bdd;
mod util;
mod worker;

use crate::util::trans_expr_to_ltl;
use automata::BuchiAutomata;
use smv::bdd::{SmvTransBdd, SmvTransBddMethod};
use smv::{bdd::SmvBdd, Expr, Prefix, Smv};
use std::{mem::take, process::Command, thread::spawn, time::Instant};
use worker::Worker;

#[cfg(feature = "peabody")]
type BddManager = peabody::Peabody;
#[cfg(feature = "peabody")]
type Bdd = peabody::Bdd;
#[cfg(feature = "cudd")]
type BddManager = cudd::Cudd;
#[cfg(feature = "cudd")]
type Bdd = cudd::Bdd;

struct PartitionedSmc {
    manager: BddManager,
    trans: SmvTransBdd<BddManager>,
    init: Bdd,
    automata: BuchiAutomata,
    workers: Vec<Worker>,
    parallel: bool,
}

impl PartitionedSmc {
    fn new(
        manager: BddManager,
        trans: SmvTransBdd<BddManager>,
        init: Bdd,
        automata: BuchiAutomata,
        parallel: bool,
    ) -> Self {
        let mut workers = Vec::new();
        if parallel {
            workers = Worker::create_workers(&trans, &automata);
        }
        Self {
            manager,
            trans,
            init,
            automata,
            workers,
            parallel,
        }
    }

    fn pre_image(&mut self, from: &[Bdd], constraint: Option<&[Bdd]>) -> Vec<Bdd> {
        assert!(from.len() == self.automata.num_state());
        let mut frontier = from.to_vec();
        let mut reach = vec![self.manager.constant(false); self.automata.num_state()];
        let mut y = 0;
        loop {
            y += 1;
            dbg!(y);
            let mut new_frontier = vec![self.manager.constant(false); self.automata.num_state()];
            let image: Vec<Bdd> = frontier.iter().map(|x| self.trans.pre_image(x)).collect();
            for i in 0..frontier.len() {
                for (next, label) in self.automata.backward[i].iter() {
                    let mut update = &image[i] & &label;
                    update &= !&reach[*next];
                    if let Some(constraint) = constraint {
                        update &= &constraint[*next];
                    }
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

    fn post_image(&mut self, from: &[Bdd]) -> Vec<Bdd> {
        assert!(from.len() == self.automata.num_state());
        let automata_trans = self.automata.forward.clone();
        let mut frontier = from.to_vec();
        let mut reach = vec![self.manager.constant(false); self.automata.num_state()];
        let mut y = 0;
        loop {
            y += 1;
            dbg!(y);
            let mut new_frontier = vec![self.manager.constant(false); self.automata.num_state()];
            let mut tmp = vec![self.manager.constant(false); self.automata.num_state()];
            for i in 0..frontier.len() {
                for (next, label) in automata_trans[i].iter() {
                    let update = &frontier[i] & label;
                    tmp[*next] |= update;
                }
            }
            let image: Vec<Bdd> = tmp.iter().map(|x| self.trans.post_image(x)).collect();
            for i in 0..image.len() {
                let update = &image[i] & !&reach[i];
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

    fn parallel_reachable_state(
        &mut self,
        from: &[Bdd],
        forward: bool,
        constraint: Option<&[Bdd]>,
    ) -> Vec<Bdd> {
        assert!(from.len() == self.workers.len());
        let workers = take(&mut self.workers);
        let mut joins = Vec::new();
        for (i, mut worker) in workers.into_iter().enumerate() {
            let from = from[i].clone();
            let constraint = constraint.map(|constraint| constraint[i].clone());
            joins.push(spawn(move || {
                let reach = worker.start(forward, from, constraint);
                (reach, worker)
            }));
        }
        let mut reach = Vec::new();
        for join in joins {
            let (image, worker) = join.join().unwrap();
            self.workers.push(worker);
            reach.push(self.manager.translocate(&image));
        }
        reach
    }

    fn fair_states(&mut self, init_reach: &[Bdd]) -> Vec<Bdd> {
        let mut fair_states = vec![self.manager.constant(false); self.automata.num_state()];
        for state in self.automata.accepting_states.iter() {
            fair_states[*state] = init_reach[*state].clone();
            // fair_states[*state] = self.manager.constant(true);
        }
        let mut x = 0;
        loop {
            x += 1;
            dbg!(x);
            let backward = if self.parallel {
                self.parallel_reachable_state(&fair_states, false, Some(init_reach))
            } else {
                // self.pre_image(&fair_states, None)
                self.pre_image(&fair_states, Some(init_reach))
            };
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
        let mut reach = vec![self.manager.constant(false); self.automata.num_state()];
        for init_state in self.automata.init_states.iter() {
            reach[*init_state] |= &self.init;
        }
        let forward = if self.parallel {
            self.parallel_reachable_state(&reach, true, None)
        } else {
            self.post_image(&reach)
        };
        for i in 0..forward.len() {
            reach[i] = &forward[i] | &reach[i];
        }
        let fair_states = self.fair_states(&reach);
        for accept in self.automata.accepting_states.iter() {
            if &reach[*accept] & &fair_states[*accept] != self.manager.constant(false) {
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
    // let smv = Smv::from_file("../MC-Benchmark/examples/counter/2bit/counter-flat.smv").unwrap();

    // let smv = Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/abp/abp8-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/abp/abp4-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/dme/dme3-flat.smv").unwrap();
    let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/prod-cons/prod-cons-flat.smv").unwrap();
    // let smv =
    // Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/example_cmu/dme1-flat.smv").unwrap();
    // let smv =
    // Smv::from_file("../MC-Benchmark/LMCS-2006/dme/dme3-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/production-cell/production-cell-flat.smv")
    // .unwrap();

    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/arbi0s08bugp03-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cutarb8ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cutf3ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cuhanoi7ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cuhanoi10ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cujc12ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cunim1ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/arbixs08bugp03-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/single/shift1add262144-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/single/bj08amba2g1-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/single/ringp0-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc19/single/aig/goel/industry/cal9/cal9-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc08/viscoherencep1-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc08/viscoherencep2-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc08/viscoherencep5-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc08/pdtvisvending00-flat.smv").unwrap();

    let manager = BddManager::new();
    let smv_bdd = SmvBdd::new(&manager, &smv, SmvTransBddMethod::Monolithic, &[]);
    dbg!(&smv.trans.len());

    let mut trans_ltl = Expr::LitExpr(true);
    // trans_ltl = trans_ltl & trans_expr_to_ltl(&smv.trans[0]);
    // trans_ltl = trans_ltl & trans_expr_to_ltl(&smv.trans[1]);
    // trans_ltl = trans_ltl & trans_expr_to_ltl(&smv.trans[2]);
    println!("{}", trans_ltl);
    let mut fairness = Expr::LitExpr(true);
    for fair in smv.fairness.iter() {
        let fair = Expr::PrefixExpr(
            Prefix::LtlGlobally,
            Box::new(Expr::PrefixExpr(Prefix::LtlFinally, Box::new(fair.clone()))),
        );
        fairness = fairness & fair;
    }

    let manager = smv_bdd.manager.clone();

    let ltl = &smv.ltlspecs[0];
    let ltl = !ltl.clone() & fairness.clone() & trans_ltl.clone();
    let ba = BuchiAutomata::from_ltl(ltl, &manager, &smv_bdd.symbols, &smv_bdd.defines);
    let mut partitioned_smc = PartitionedSmc::new(
        manager.clone(),
        smv_bdd.trans.clone(),
        smv_bdd.init.clone(),
        ba,
        true,
    );
    let start = Instant::now();
    dbg!(partitioned_smc.check());
    println!("{:?}", start.elapsed());
}
