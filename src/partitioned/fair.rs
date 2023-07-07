use super::PartitionedSmc;
use crate::Bdd;
use sylvan::LaceWorkerContext;

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
            // self.pre_reachable(&fair_states, None)
            let backward = self.pre_reachable(&fair_states, Some(init_reach));
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

    pub fn lace_fair_states(
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
            let backward = self.lace_pre_reachable(context, &fair_states, Some(init_reach));
            fair_states.iter().zip(backward.iter()).for_each(|(x, y)| {
                let x = x.clone();
                let y = y.clone();
                context.lace_spawn(|_| x & y)
            });
            let new_fair_states: Vec<Bdd> = context.lace_sync_multi(fair_states.len());
            if fair_states == new_fair_states {
                break;
            }
            fair_states = new_fair_states;
        }
        fair_states
    }
}
