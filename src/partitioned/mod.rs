mod reachable;
mod worker;

use crate::{automata::BuchiAutomata, Bdd, BddManager};
use fsmbdd::FsmBdd;
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

pub fn check(
    manager: BddManager,
    fsmbdd: FsmBdd<BddManager>,
    ba: BuchiAutomata,
    parallel: bool,
) -> Duration {
    let mut partitioned_smc = PartitionedSmc::new(manager.clone(), fsmbdd, ba, parallel);
    let start = Instant::now();
    dbg!(partitioned_smc.check());
    start.elapsed()
}
