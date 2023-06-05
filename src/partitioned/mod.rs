mod reachable;
mod worker;

use crate::{
    automata::BuchiAutomata, command::Args, ltl::ltl_to_automata_preprocess, Bdd, BddManager,
};
use fsmbdd::FsmBdd;
use smv::{bdd::SmvBdd, Expr, Prefix, Smv};
use std::time::{Duration, Instant};
use worker::Worker;

pub struct PartitionedSmc {
    manager: BddManager,
    fsmbdd: FsmBdd<BddManager>,
    automata: BuchiAutomata,
    workers: Vec<Worker>,
    parallel: bool,
}

impl PartitionedSmc {
    pub fn new(
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

    pub fn check(&mut self) -> bool {
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

fn get_ltl(smv: &Smv, extend_trans: &[usize]) -> Expr {
    dbg!(&smv.trans.len());
    dbg!(extend_trans);
    // let smv = smv.flatten_defines();
    let trans_ltl = extend_trans
        .iter()
        .fold(Expr::LitExpr(true), |fold, extend| {
            fold & Expr::PrefixExpr(Prefix::LtlGlobally, Box::new(smv.trans[*extend].clone()))
        });
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
    let ltl = ltl_to_automata_preprocess(&smv, ltl);
    println!("{}", ltl);
    ltl
}

pub fn check(manager: BddManager, smv: Smv, args: Args) -> (bool, Duration) {
    let smv_bdd = SmvBdd::new(&manager, &smv);
    let mut fsmbdd = smv_bdd.to_fsmbdd(args.trans_method.into());
    fsmbdd.justice.clear();
    let ba = BuchiAutomata::from_ltl(
        get_ltl(&smv, &args.ltl_extend_trans),
        &manager,
        &smv_bdd.symbols,
        &smv_bdd.defines,
    );
    let mut partitioned_smc = PartitionedSmc::new(manager.clone(), fsmbdd, ba, args.parallel);
    let start = Instant::now();
    (partitioned_smc.check(), start.elapsed())
}
