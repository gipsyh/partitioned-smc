mod other;
mod slice;
mod worker;

use self::{
    slice::{Slice, SliceManager},
    worker::Worker,
};
use crate::{
    automata::BuchiAutomata, command::Args, ltl::ltl_to_automata_preprocess, Bdd, BddManager,
};
use fsmbdd::FsmBdd;
use smv::{bdd::SmvBdd, Smv};
use std::{
    mem::take,
    sync::Arc,
    thread::spawn,
    time::{Duration, Instant},
};

pub struct Cav00 {
    manager: BddManager,
    fsmbdd: FsmBdd<BddManager>,
    init_slice: Vec<Slice>,
    slice_manager: Arc<SliceManager>,
    workers: Vec<Worker>,
}

impl Cav00 {
    fn new(
        manager: BddManager,
        num_worker: usize,
        fsmbdd: FsmBdd<BddManager>,
        slice_var: &[usize],
    ) -> Self {
        let mut init_slice = vec![Slice::new([])];
        for var in slice_var {
            let mut neg = init_slice.clone();
            for i in 0..init_slice.len() {
                init_slice[i].push((*var, true));
                neg[i].push((*var, false));
            }
            init_slice.extend(neg);
        }
        let slice_manager = Arc::new(SliceManager::new(init_slice.clone(), num_worker));
        let workers = Worker::create_workers(&fsmbdd, num_worker, slice_manager.clone());
        Self {
            manager,
            fsmbdd,
            init_slice,
            slice_manager,
            workers,
        }
    }

    fn reachable(
        &mut self,
        from: Bdd,
        forward: bool,
        constraint: Option<Bdd>,
        contain_from: bool,
    ) -> Bdd {
        self.slice_manager.reset_slice(self.init_slice.clone());
        let mut workers = take(&mut self.workers);
        let mut joins = Vec::new();
        for i in 0..self.init_slice.len() {
            workers[i].reset(Some(self.init_slice[i].clone()));
        }
        for i in self.init_slice.len()..workers.len() {
            workers[i].reset(None);
        }
        for (id, mut worker) in workers.into_iter().enumerate() {
            let from = if id < self.init_slice.len() {
                from.clone() & self.init_slice[id].bdd(&self.manager)
            } else {
                self.manager.constant(false)
            };
            let constraint = constraint.clone();
            joins.push(spawn(move || {
                let reach = worker.reachable(from, forward, constraint);
                (reach, worker)
            }));
        }
        let mut reach = self.manager.constant(false);
        for join in joins {
            let (image, worker) = join.join().unwrap();
            self.workers.push(worker);
            reach |= self.manager.translocate(&image);
        }
        if contain_from {
            reach |= from;
        }
        reach
    }

    pub fn fair_cycle_with_constrain(&mut self, constrain: Bdd) -> Bdd {
        let mut res = constrain.clone();
        let mut y = 0;
        loop {
            y += 1;
            dbg!(y);
            let mut new = res.clone();
            for fair in self.fsmbdd.justice.clone().iter() {
                let fair = fair & &res;
                let backward = self.reachable(fair, false, Some(constrain.clone()), false);
                new &= backward;
            }
            if new == res {
                break res;
            }
            res = new;
        }
    }

    fn check(&mut self) -> bool {
        let forward = self.reachable(self.fsmbdd.init.clone(), true, None, true);
        let fair_cycle = self.fair_cycle_with_constrain(forward.clone());
        (fair_cycle & forward).is_constant(false)
    }
}

pub fn check(manager: BddManager, smv: Smv, args: Args) -> (bool, Duration) {
    let smvbdd = SmvBdd::new(&manager, &smv);
    let fsmbdd = smvbdd.to_fsmbdd(args.trans_method.into());
    let ltl = ltl_to_automata_preprocess(&smv, !smv.ltlspecs[0].clone());
    let ltl_fsmbdd =
        BuchiAutomata::from_ltl(ltl, &manager, &smvbdd.symbols, &smvbdd.defines).to_fsmbdd();
    let product = fsmbdd.product(&ltl_fsmbdd);
    let mut cav00 = Cav00::new(manager, 8, product, &[0, 1, 2]);
    let start = Instant::now();
    (cav00.check(), start.elapsed())
}
