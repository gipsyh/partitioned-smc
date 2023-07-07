use crate::{automata::BuchiAutomata, Bdd, BddManager};
use fsmbdd::FsmBdd;

#[allow(dead_code)]
pub struct Worker {
    id: usize,
    manager: BddManager,
    fsmbdd: FsmBdd<BddManager>,
    forward: Vec<(usize, Bdd)>,
    backward: Vec<(usize, Bdd)>,
}

#[allow(dead_code)]
impl Worker {
    #[inline]
    #[allow(unused)]
    fn propagate_from(&self, reach: &Bdd, data: &[Bdd], constraint: Option<Bdd>) -> (Bdd, Bdd) {
        let mut new_frontier = self.manager.constant(false);
        let mut reach = reach.clone();
        for (from, label) in self.forward.iter() {
            let update = &data[*from] & label;
            // if let
            // & !&reach;
            todo!();
            reach |= &update;
            new_frontier |= update;
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
