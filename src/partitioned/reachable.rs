use super::PartitionedSmc;
use crate::Bdd;
use std::time::Instant;
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
        let mut reach = vec![self.manager.constant(false); self.automata.num_state()];
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

    // pub fn parallel_reachable_state(
    //     &mut self,
    //     from: &[Bdd],
    //     forward: bool,
    //     constraint: Option<&[Bdd]>,
    // ) -> Vec<Bdd> {
    //     assert!(from.len() == self.workers.len());
    //     let mut workers = take(&mut self.workers);
    //     let mut joins = Vec::new();
    //     for worker in workers.iter_mut() {
    //         worker.reset();
    //     }
    //     for (i, mut worker) in workers.into_iter().enumerate() {
    //         let init = from[i].clone();
    //         let constraint = constraint.map(|constraint| constraint[i].clone());
    //         joins.push(spawn(move || {
    //             let reach = worker.reachable(forward, init, constraint);
    //             (reach, worker)
    //         }));
    //     }
    //     let mut reach = Vec::new();
    //     for join in joins {
    //         let (image, worker) = join.join().unwrap();
    //         self.workers.push(worker);
    //         reach.push(self.manager.translocate(&image));
    //     }
    //     reach
    // }
}

impl PartitionedSmc {
    pub fn lace_post_reachable(
        &mut self,
        mut context: LaceWorkerContext,
        from: &[Bdd],
    ) -> Vec<Bdd> {
        let mut frontier = from.to_vec();
        let partitioned_len = from.len();
        let mut reach = vec![self.manager.constant(false); partitioned_len];
        let mut post_deep = 0;
        loop {
            post_deep += 1;
            if self.args.verbose {
                dbg!(post_deep);
            }
            let mut tmp = vec![self.manager.constant(false); partitioned_len];
            for i in 0..partitioned_len {
                for (next, label) in self.automata.forward[i].iter() {
                    let update = &frontier[i] & label;
                    tmp[*next] |= update;
                }
            }
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
            frontier.iter().for_each(|x| {
                let fsmbdd = self.fsmbdd.clone();
                let x = x.clone();
                context.lace_spawn(move |_| fsmbdd.pre_image(&x));
            });
            let image: Vec<Bdd> = context.lace_sync_multi(partitioned_len);
            self.statistic.image_time += start.elapsed();
            let mut new_frontier = vec![self.manager.constant(false); partitioned_len];
            let start = Instant::now();
            for i in 0..partitioned_len {
                for (next, label) in self.automata.backward[i].iter() {
                    let start_a = Instant::now();
                    let mut update = &image[i] & &label;
                    self.statistic.propagate_time_a += start_a.elapsed();
                    if let Some(constraint) = constraint {
                        update &= &constraint[*next];
                    }
                    update &= !&reach[*next];
                    new_frontier[*next] = &new_frontier[*next] | &update;
                    reach[*next] = &reach[*next] | update;
                }
            }
            self.statistic.propagate_time += start.elapsed();
            if new_frontier.iter().all(|bdd| bdd.is_constant(false)) {
                break;
            }
            frontier = new_frontier;
        }
        reach
    }
}
