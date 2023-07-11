use super::PartitionedSmc;
use crate::Bdd;
use std::{sync::Arc, time::Instant};
use sylvan::LaceWorkerContext;

impl PartitionedSmc {
    pub fn pre_reachable(&mut self, from: &[Bdd], constraint: Option<&[Bdd]>) -> Vec<Bdd> {
        assert!(from.len() == self.automata.num_state());
        let mut frontier = from.to_vec();
        let mut reach = vec![self.manager.constant(false); self.automata.num_state()];
        let mut y = 0;
        loop {
            y += 1;
            if self.args.verbose {
                dbg!(y);
            }
            let mut new_frontier = vec![self.manager.constant(false); self.automata.num_state()];
            let image: Vec<Bdd> = frontier.iter().map(|x| self.fsmbdd.pre_image(x)).collect();
            for i in 0..frontier.len() {
                for (next, label) in self.automata.backward[i].iter() {
                    let mut update = &image[i] & &label;
                    if let Some(constraint) = constraint {
                        update &= &constraint[*next];
                    }
                    update &= !&reach[*next];
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

    pub fn post_reachable(&mut self, from: &[Bdd]) -> Vec<Bdd> {
        assert!(from.len() == self.automata.num_state());
        let mut frontier = from.to_vec();
        let mut reach = frontier.clone();
        let mut post_deep = 0;
        loop {
            post_deep += 1;
            if self.args.verbose {
                dbg!(post_deep);
            }
            let mut new_frontier = vec![self.manager.constant(false); self.automata.num_state()];
            let mut tmp = vec![self.manager.constant(false); self.automata.num_state()];
            for i in 0..frontier.len() {
                for (next, label) in self.automata.forward[i].iter() {
                    let update = &frontier[i] & label;
                    tmp[*next] |= update;
                }
            }
            let image: Vec<Bdd> = tmp.iter().map(|x| self.fsmbdd.post_image(x)).collect();
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
}

impl PartitionedSmc {
    fn lace_propagate(
        &self,
        mut context: LaceWorkerContext,
        _forward: bool,
        states: &[Bdd],
        reach: &[Bdd],
        constraint: Option<&[Bdd]>,
    ) -> (Vec<Bdd>, Vec<Bdd>) {
        let partitioned_len = states.len();
        let states = Arc::new(states.to_vec());
        for i in 0..partitioned_len {
            let worker = self.workers[i].clone();
            let reach = reach[i].clone();
            let states = states.clone();
            let constraint = constraint.map(|c| c[i].clone());
            context.lace_spawn(move |_| worker.propagate(reach, states, constraint))
        }
        let res = context.lace_sync_multi::<(Bdd, Bdd)>(partitioned_len);
        let mut reach = Vec::new();
        let mut new_frontier = Vec::new();
        for (r, f) in res.into_iter() {
            reach.push(r);
            new_frontier.push(f);
        }
        (reach, new_frontier)
    }
}

impl PartitionedSmc {
    pub fn lace_post_reachable(
        &mut self,
        mut context: LaceWorkerContext,
        from: &[Bdd],
    ) -> Vec<Bdd> {
        let mut frontier = from.to_vec();
        let partitioned_len = from.len();
        let mut reach = frontier.clone();
        let mut post_deep = 0;
        loop {
            post_deep += 1;
            if self.args.verbose {
                dbg!(post_deep);
            }
            let start = Instant::now();
            let mut tmp = vec![self.manager.constant(false); partitioned_len];
            for i in 0..partitioned_len {
                for (next, label) in self.automata.forward[i].iter() {
                    let update = &frontier[i] & label;
                    tmp[*next] |= update;
                }
            }
            self.statistic.post_propagate_time += start.elapsed();
            let start = Instant::now();
            for i in 0..partitioned_len {
                let bdd = tmp[i].clone();
                let fsmbdd = self.fsmbdd.clone();
                let mut reach = reach[i].clone();
                context.lace_spawn(move |_| {
                    let image = fsmbdd.post_image(&bdd);
                    let update = &image & !&reach;
                    reach |= &update;
                    (reach, update)
                });
            }
            let reach_update: Vec<(Bdd, Bdd)> = context.lace_sync_multi(partitioned_len);
            self.statistic.post_image_time += start.elapsed();
            let mut new_frontier = Vec::new();
            reach = Vec::new();
            for (reach_bdd, update) in reach_update {
                reach.push(reach_bdd);
                new_frontier.push(update);
            }
            if new_frontier.iter().all(|bdd| bdd.is_constant(false)) {
                break reach;
            }
            frontier = new_frontier;
        }
    }

    pub fn lace_pre_reachable(
        &mut self,
        mut context: LaceWorkerContext,
        from: &[Bdd],
        constraint: Option<&[Bdd]>,
    ) -> Vec<Bdd> {
        assert!(from.len() == self.automata.num_state());
        let partitioned_len = from.len();
        let mut frontier = from.to_vec();
        let mut reach = vec![self.manager.constant(false); partitioned_len];
        let mut y = 0;
        loop {
            y += 1;
            if self.args.verbose {
                dbg!(y);
            }
            let start = Instant::now();
            for i in 0..partitioned_len {
                let worker = self.workers[i].clone();
                let x = frontier[i].clone();
                context.lace_spawn(move |_| worker.fsmbdd.pre_image(&x));
            }
            let image: Vec<Bdd> = context.lace_sync_multi(partitioned_len);
            self.statistic.pre_image_time += start.elapsed();
            let start = Instant::now();
            let new_frontier;
            (reach, new_frontier) = self.lace_propagate(context, false, &image, &reach, constraint);
            self.statistic.pre_propagate_time += start.elapsed();
            if new_frontier.iter().all(|bdd| bdd.is_constant(false)) {
                break;
            }
            frontier = new_frontier;
        }
        reach
    }
}
