mod automata;
mod bdd;
mod util;
mod worker;

use crate::util::trans_expr_to_ltl;
use automata::BuchiAutomata;
use cudd::{Cudd, DdNode};
use smv::bdd::{SmvTransBdd, SmvTransBddMethod};
use smv::{bdd::SmvBdd, Expr, Prefix, Smv};
use std::{
    mem::take,
    process::Command,
    thread::spawn,
    time::{Duration, Instant},
};
use worker::Worker;

struct PartitionedSmc {
    cudd: Cudd,
    trans: SmvTransBdd,
    init: DdNode,
    automata: BuchiAutomata,
    workers: Vec<Worker>,
    parallel: bool,
    and_time: Duration,
    image_time: Duration,
}

impl PartitionedSmc {
    fn new(
        cudd: Cudd,
        trans: SmvTransBdd,
        init: DdNode,
        automata: BuchiAutomata,
        parallel: bool,
    ) -> Self {
        let mut workers = Vec::new();
        if parallel {
            workers = Worker::create_workers(&trans, &automata);
        }
        Self {
            cudd,
            trans,
            init,
            automata,
            workers,
            parallel,
            and_time: Duration::default(),
            image_time: Duration::default(),
        }
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
            let image: Vec<DdNode> = frontier
                .iter()
                .map(|x| {
                    if forward {
                        self.trans.post_image(x)
                    } else {
                        self.trans.pre_image(x)
                    }
                })
                .collect();
            for i in 0..frontier.len() {
                for (next, label) in automata_trans[i].iter_mut() {
                    *label = label.as_ref() & !&reach[*next];
                    if let Some(constraint) = constraint {
                        *label = label.as_ref() & &constraint[*next];
                    }
                    let update = &image[i] & &label;
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
            let image: Vec<DdNode> = tmp
                .iter()
                .map(|x| {
                    if forward {
                        self.trans.post_image(x)
                    } else {
                        self.trans.pre_image(x)
                    }
                })
                .collect();
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
        from: &[DdNode],
        forward: bool,
        constraint: Option<&[DdNode]>,
    ) -> Vec<DdNode> {
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
            reach.push(self.cudd.translocate(&image));
        }
        reach
    }

    fn fair_states(&mut self, init_reach: &[DdNode]) -> Vec<DdNode> {
        let mut fair_states = vec![self.cudd.constant(false); self.automata.num_state()];
        for state in self.automata.accepting_states.iter() {
            fair_states[*state] = init_reach[*state].clone();
        }
        let mut x = 0;
        loop {
            x += 1;
            dbg!(x);
            let backward = if self.parallel {
                self.parallel_reachable_state(&fair_states, false, Some(init_reach))
            } else {
                self.reachable_state_image_first(&fair_states, false, Some(init_reach))
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
        let mut reach = vec![self.cudd.constant(false); self.automata.num_state()];
        for init_state in self.automata.init_states.iter() {
            reach[*init_state] |= &self.init;
        }
        let forward = if self.parallel {
            self.parallel_reachable_state(&reach, true, None)
        } else {
            self.reachable_state_propagate_first(&reach, true)
        };
        for i in 0..forward.len() {
            reach[i] = &forward[i] | &reach[i];
        }
        let fair_states = self.fair_states(&reach);
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
    // let smv = Smv::from_file("../MC-Benchmark/examples/counter/2bit/counter-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/abp/abp8-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/abp/abp4-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/dme/dme3-flat.smv").unwrap();
    // let smv =
    // Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/example_cmu/dme1-flat.smv").unwrap();
    // let smv =
    // Smv::from_file("../MC-Benchmark/LMCS-2006/dme/dme3-flat.smv").unwrap();
    let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/prod-cons/prod-cons-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/production-cell/production-cell-flat.smv")
    // .unwrap();
    // let smv = Smv::from_file("../ATVA/trp/N12x/1/pltl-12-0-1-3-0-200000.smv").unwrap();

    let smv_bdd = SmvBdd::new(&smv, SmvTransBddMethod::Monolithic);
    // dbg!(&smv_bdd.cudd);
    // dbg!(&smv_bdd.trans);
    // dbg!(&smv_bdd.symbols);
    // dbg!(&smv_bdd.init);
    // dbg!(&smv.trans[]);
    let mut trans_ltl = Expr::LitExpr(true);
    for tran in &smv.trans[0..1] {
        trans_ltl = trans_ltl & trans_expr_to_ltl(tran);
    }
    println!("{}", trans_ltl);
    let mut fairness = Expr::LitExpr(true);
    for fair in smv.fairness.iter() {
        let fair = Expr::PrefixExpr(
            Prefix::LtlGlobally,
            Box::new(Expr::PrefixExpr(Prefix::LtlFinally, Box::new(fair.clone()))),
        );
        fairness = fairness & fair;
    }
    let mut cudd = smv_bdd.cudd.clone();

    // for _ in 0..5 {
    for ltl in &smv.ltlspecs[..] {
        let ltl = !ltl.clone() & fairness.clone() & trans_ltl.clone();
        println!("'{}'", ltl);
        let ltl2dfa = Command::new("/root/ltl2ba-1.3/ltl2ba")
            .arg("-f")
            .arg(format!("{}", ltl))
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
            true,
        );
        let start = Instant::now();
        dbg!(partitioned_smc.check());
        println!("{:?}", start.elapsed());
        dbg!(partitioned_smc.and_time);
        dbg!(partitioned_smc.image_time);
    }
    // }
}
