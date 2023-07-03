use super::PartitionedSmc;
use crate::Bdd;
use std::iter::repeat_with;
use sylvan::{lace_call_back, LaceCallback, LaceWorkerContext};

struct LaceFairCallbackArg<'a> {
    partitioned_smc: &'a mut PartitionedSmc,
    init_reach: &'a [Bdd],
}

pub struct LaceFairCallback;

impl LaceCallback<LaceFairCallbackArg<'_>, Vec<Bdd>> for LaceFairCallback {
    fn callback(context: LaceWorkerContext, arg: &mut LaceFairCallbackArg) -> Vec<Bdd> {
        arg.partitioned_smc
            .lace_fair_states_inner(context, arg.init_reach)
    }
}

impl PartitionedSmc {
    pub fn fair_states(&mut self, init_reach: &[Bdd]) -> Vec<Bdd> {
        let mut fair_states = vec![self.manager.constant(false); self.automata.num_state()];
        for state in self.automata.accepting_states.iter() {
            fair_states[*state] = init_reach[*state].clone();
            // fair_states[*state] = self.manager.constant(true);
        }
        let mut x = 0;
        loop {
            x += 1;
            dbg!(x);
            let backward = if self.args.parallel {
                self.parallel_reachable_state(&fair_states, false, Some(init_reach))
            } else {
                // self.pre_reachable(&fair_states, None)
                if self.args.close_lace_optimize {
                    self.pre_reachable(&fair_states, Some(init_reach))
                } else {
                    self.lace_pre_reachable(&fair_states, Some(init_reach))
                }
            };
            let mut new_fair_states = Vec::new();
            for i in 0..fair_states.len() {
                new_fair_states.push(&fair_states[i] & &backward[i]);
            }
            if fair_states == new_fair_states {
                break;
            }
            fair_states = new_fair_states;
        }
        fair_states
    }

    fn lace_fair_states_inner(
        &mut self,
        mut context: LaceWorkerContext,
        init_reach: &[Bdd],
    ) -> Vec<Bdd> {
        let mut fair_states = vec![self.manager.constant(false); self.automata.num_state()];
        for state in self.automata.accepting_states.iter() {
            fair_states[*state] = init_reach[*state].clone();
            // fair_states[*state] = self.manager.constant(true);
        }
        let mut x = 0;
        loop {
            x += 1;
            dbg!(x);
            let backward = if self.args.parallel {
                self.parallel_reachable_state(&fair_states, false, Some(init_reach))
            } else {
                // self.pre_reachable(&fair_states, None)
                if self.args.close_lace_optimize {
                    self.pre_reachable(&fair_states, Some(init_reach))
                } else {
                    self.lace_pre_reachable(&fair_states, Some(init_reach))
                }
            };
            fair_states
                .iter()
                .zip(backward.iter())
                .for_each(|(x, y)| context.spawn_and(x, y));
            let mut new_fair_states: Vec<Bdd> = repeat_with(|| context.sync_and())
                .take(fair_states.len())
                .collect();
            new_fair_states.reverse();
            // let mut new_fair_states = Vec::new();
            // for i in 0..fair_states.len() {
            //     new_fair_states.push(&fair_states[i] & &backward[i]);
            // }
            if fair_states == new_fair_states {
                break;
            }
            fair_states = new_fair_states;
        }
        fair_states
    }

    pub fn lace_fair_states(&mut self, init_reach: &[Bdd]) -> Vec<Bdd> {
        let mut arg = LaceFairCallbackArg {
            partitioned_smc: self,
            init_reach,
        };
        lace_call_back::<LaceFairCallback, LaceFairCallbackArg, Vec<Bdd>>(&mut arg)
    }
}
