use super::PartitionedSmc;
use crate::Bdd;
use std::{iter::repeat_with, mem::take, thread::spawn};
use sylvan::{lace_call_back, LaceCallback, LaceWorkerContext};

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

    pub fn parallel_reachable_state(
        &mut self,
        from: &[Bdd],
        forward: bool,
        constraint: Option<&[Bdd]>,
    ) -> Vec<Bdd> {
        assert!(from.len() == self.workers.len());
        let mut workers = take(&mut self.workers);
        let mut joins = Vec::new();
        for worker in workers.iter_mut() {
            worker.reset();
        }
        for (i, mut worker) in workers.into_iter().enumerate() {
            let init = from[i].clone();
            let constraint = constraint.map(|constraint| constraint[i].clone());
            joins.push(spawn(move || {
                let reach = worker.reachable(forward, init, constraint);
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
}

struct LaceReachableCallbackArg<'a> {
    partitioned_smc: &'a mut PartitionedSmc,
    constraint: Option<&'a [Bdd]>,
    from: &'a [Bdd],
}

pub struct LacePostReachableCallback;

impl LaceCallback<LaceReachableCallbackArg<'_>, Vec<Bdd>> for LacePostReachableCallback {
    fn callback(context: LaceWorkerContext, arg: &mut LaceReachableCallbackArg) -> Vec<Bdd> {
        arg.partitioned_smc
            .lace_post_reachable_inner(context, arg.from)
    }
}

pub struct LacePreReachableCallback;

impl LaceCallback<LaceReachableCallbackArg<'_>, Vec<Bdd>> for LacePreReachableCallback {
    fn callback(context: LaceWorkerContext, arg: &mut LaceReachableCallbackArg) -> Vec<Bdd> {
        arg.partitioned_smc
            .lace_pre_reachable_inner(context, arg.from, arg.constraint)
    }
}

impl PartitionedSmc {
    fn lace_post_reachable_inner(
        &mut self,
        mut context: LaceWorkerContext,
        from: &[Bdd],
    ) -> Vec<Bdd> {
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
            tmp.iter()
                .for_each(|x| self.fsmbdd.lace_spawn_post_image(&mut context, x));
            let mut image: Vec<Bdd> =
                repeat_with(|| self.fsmbdd.lace_sync_post_image(&mut context))
                    .take(tmp.len())
                    .collect();
            image.reverse();
            // let image: Vec<Bdd> = tmp.iter().map(|x| self.fsmbdd.post_image(x)).collect();
            for i in 0..image.len() {
                let update = &image[i] & !&reach[i];
                reach[i] |= &update;
                new_frontier[i] |= update;
            }
            if new_frontier.iter().all(|bdd| bdd.is_constant(false)) {
                break reach;
            }
            frontier = new_frontier;
        }
    }

    pub fn lace_post_reachable(&mut self, from: &[Bdd]) -> Vec<Bdd> {
        let mut arg = LaceReachableCallbackArg {
            partitioned_smc: self,
            constraint: None,
            from,
        };
        lace_call_back::<LacePostReachableCallback, LaceReachableCallbackArg, Vec<Bdd>>(&mut arg)
    }

    fn lace_pre_reachable_inner(
        &mut self,
        mut context: LaceWorkerContext,
        from: &[Bdd],
        constraint: Option<&[Bdd]>,
    ) -> Vec<Bdd> {
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
            // let image: Vec<Bdd> = frontier.iter().map(|x| self.fsmbdd.pre_image(x)).collect();
            frontier
                .iter()
                .for_each(|x| self.fsmbdd.lace_spawn_pre_image(&mut context, x));
            let mut image: Vec<Bdd> = repeat_with(|| self.fsmbdd.lace_sync_pre_image(&mut context))
                .take(frontier.len())
                .collect();
            image.reverse();
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

    pub fn lace_pre_reachable(&mut self, from: &[Bdd], constraint: Option<&[Bdd]>) -> Vec<Bdd> {
        let mut arg = LaceReachableCallbackArg {
            partitioned_smc: self,
            constraint,
            from,
        };
        lace_call_back::<LacePreReachableCallback, LaceReachableCallbackArg, Vec<Bdd>>(&mut arg)
    }
}
