use std::{mem::take, thread::spawn};

use crate::{Bdd, PartitionedSmc};

impl PartitionedSmc {
    pub fn pre_reachable(&mut self, from: &[Bdd], constraint: Option<&[Bdd]>) -> Vec<Bdd> {
        assert!(from.len() == self.automata.num_state());
        let mut frontier = from.to_vec();
        let mut reach = vec![self.manager.constant(false); self.automata.num_state()];
        let mut y = 0;
        loop {
            y += 1;
            dbg!(y);
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
            dbg!(post_deep);
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
        for (i, worker) in workers.iter_mut().enumerate() {
            worker.init(forward, from[i].clone())
        }
        for (i, mut worker) in workers.into_iter().enumerate() {
            let constraint = constraint.map(|constraint| constraint[i].clone());
            joins.push(spawn(move || {
                let reach = worker.start(forward, constraint);
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