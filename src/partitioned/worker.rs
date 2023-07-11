use crate::{automata::BuchiAutomata, Bdd, BddManager};
use fsmbdd::FsmBdd;
use std::sync::Arc;

#[allow(dead_code)]
pub struct Worker {
    id: usize,
    manager: BddManager,
    pub fsmbdd: FsmBdd<BddManager>,
    forward: Vec<(usize, Bdd)>,
    backward: Vec<(usize, Bdd)>,
}

#[allow(dead_code)]
impl Worker {
    pub fn propagate(
        &self,
        reach: Bdd,
        data: Arc<Vec<Bdd>>,
        constraint: Option<Bdd>,
    ) -> (Bdd, Bdd) {
        let mut new_frontier = self.manager.constant(false);
        let mut reach = reach.clone();
        for (from, label) in self.forward.iter() {
            let mut update = &data[*from] & &label;
            if let Some(constraint) = &constraint {
                update &= constraint;
            }
            update &= !&reach;
            new_frontier |= &update;
            reach |= update;
        }
        (reach, new_frontier)
    }

    pub fn create_workers(fsmbdd: &FsmBdd<BddManager>, automata: &BuchiAutomata) -> Vec<Self> {
        let mut workers = vec![];
        for id in 0..automata.num_state() {
            let fsmbdd = fsmbdd.clone_with_new_manager();
            let forward = automata.forward[id].clone();
            let backward = automata.backward[id].clone();
            workers.push(Self {
                id,
                manager: fsmbdd.manager.clone(),
                fsmbdd,
                forward,
                backward,
            })
        }
        workers
    }
}
