#![feature(stmt_expr_attributes)]

mod automata;
mod bdd;
mod cav00;
mod liveness;
mod reachable;
mod safety;
mod util;
mod worker;

use crate::util::trans_expr_to_ltl;
use automata::BuchiAutomata;
use fsmbdd::{FsmBdd, TransBddMethod};
use smv::{bdd::SmvBdd, Expr, Prefix, Smv};
use std::{mem::take, thread::spawn, time::Instant};
use worker::Worker;

type BddManager = cudd::Cudd;
type Bdd = cudd::Bdd;

struct PartitionedSmc {
    manager: BddManager,
    fsmbdd: FsmBdd<BddManager>,
    automata: BuchiAutomata,
    workers: Vec<Worker>,
    parallel: bool,
}

impl PartitionedSmc {
    fn new(
        manager: BddManager,
        fsmbdd: FsmBdd<BddManager>,
        automata: BuchiAutomata,
        parallel: bool,
    ) -> Self {
        let mut workers = Vec::new();
        if parallel {
            workers = Worker::create_workers(&fsmbdd, &automata);
        }
        Self {
            manager,
            fsmbdd,
            automata,
            workers,
            parallel,
        }
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
                // self.pre_reachable(&fair_states, None)
                self.pre_reachable(&fair_states, Some(init_reach))
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
            reach[*init_state] |= &self.fsmbdd.init;
        }
        let forward = if self.parallel {
            self.parallel_reachable_state(&reach, true, None)
        } else {
            self.post_reachable(&reach)
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

fn get_ltl(smv: &Smv) -> Expr {
    dbg!(&smv.trans.len());
    let mut trans_ltl = Expr::LitExpr(true);
    // trans_ltl = trans_ltl & Expr::PrefixExpr(Prefix::LtlGlobally, Box::new(smv.trans[31].clone()));
    // trans_ltl = trans_ltl & Expr::PrefixExpr(Prefix::LtlGlobally, Box::new(smv.trans[30].clone()));
    let mut fairness = Expr::LitExpr(true);
    for fair in smv.fairness.iter() {
        let fair = Expr::PrefixExpr(
            Prefix::LtlGlobally,
            Box::new(Expr::PrefixExpr(Prefix::LtlFinally, Box::new(fair.clone()))),
        );
        fairness = fairness & fair;
    }
    let ltl = smv.ltlspecs[0].clone();
    let ltl = !Expr::InfixExpr(
        smv::Infix::Imply,
        Box::new(trans_ltl & fairness),
        Box::new(ltl),
    );
    let ltl = smv.flatten_to_propositional_define(&ltl);
    let ltl = smv.flatten_case(ltl);
    let ltl = trans_expr_to_ltl(&ltl);
    println!("{}", ltl);
    ltl
}

fn main() {
    // let mut smv = Smv::from_file("../MC-Benchmark/LMCS-2006/mutex/mutex-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/short/short-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/ring/ring-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/examples/counter/2bit/counter-flat.smv").unwrap();

    let smv = Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/abp/abp8-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/abp/abp4-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/prod-cons/prod-cons-flat.smv").unwrap();
    // let smv =
    //     Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/example_cmu/dme1-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/dme/dme3-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/dme/dme3-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/production-cell/production-cell-flat.smv")
    //     .unwrap();

    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/arbi0s08bugp03-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cutarb8ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cutf3ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cuhanoi7ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cuhanoi10ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cujc12ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cunim1ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/arbixs08bugp03-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/single/shift1add262144-flat.smv").unwrap();
    // let mut smv = Smv::from_file("../MC-Benchmark/hwmcc17/single/bj08amba2g1-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/single/ringp0-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc19/single/aig/goel/industry/cal9/cal9-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc08/viscoherencep1-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc08/viscoherencep2-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc08/viscoherencep5-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc08/pdtvisvending00-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc08/bj08amba2g5-flat.smv").unwrap();

    // smv.flatten_defines();
    let manager = BddManager::new();
    let smv_bdd = SmvBdd::new(&manager, &smv, &[]);
    let fsmbdd = smv_bdd.to_fsmbdd(TransBddMethod::Partition);
    let ba = BuchiAutomata::from_ltl(get_ltl(&smv), &manager, &smv_bdd.symbols, &smv_bdd.defines);
    let mut partitioned_smc = PartitionedSmc::new(manager.clone(), fsmbdd, ba, true);
    let start = Instant::now();
    dbg!(partitioned_smc.check());
    // dbg!(partitioned_smc.check_ltl_cav00());
    // dbg!(partitioned_smc.check_ltl());
    println!("{:?}", start.elapsed());
}
