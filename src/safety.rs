use crate::{Bdd, PartitionedSmc};

impl PartitionedSmc {
    fn safety_forward(&mut self, from: &[Bdd]) -> bool {
        assert!(from.len() == self.automata.num_state());
        let mut frontier = from.to_vec();
        let mut reach = vec![self.manager.constant(false); self.automata.num_state()];
        let mut post_deep = 0;
        loop {
            post_deep += 1;
            dbg!(post_deep);
            for accept in self.automata.accepting_states.iter() {
                if reach[*accept] != self.manager.constant(false) {
                    return false;
                }
            }
            let mut tmp = vec![self.manager.constant(false); self.automata.num_state()];
            for i in 0..frontier.len() {
                for (next, label) in self.automata.forward[i].iter() {
                    let update = &frontier[i] & label;
                    tmp[*next] |= update;
                }
            }
            let image: Vec<Bdd> = tmp.iter().map(|x| self.fsmbdd.post_image(x)).collect();
            let mut new_frontier = vec![self.manager.constant(false); self.automata.num_state()];
            for i in 0..image.len() {
                let update = &image[i] & !&reach[i];
                reach[i] |= &update;
                new_frontier[i] |= update;
            }
            if new_frontier.iter().all(|bdd| bdd.is_constant(false)) {
                return true;
            }
            frontier = new_frontier;
        }
    }

    pub fn check_safety(&mut self) -> bool {
        let mut init = vec![self.manager.constant(false); self.automata.num_state()];
        for init_state in self.automata.init_states.iter() {
            init[*init_state] |= &self.fsmbdd.init;
        }
        self.safety_forward(&init)
    }
}
