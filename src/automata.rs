use cudd::{Cudd, DdNode};

pub struct BuchiAutomata {
    pub forward: Vec<Vec<(usize, DdNode)>>,
    pub backward: Vec<Vec<(usize, DdNode)>>,
    pub accepting_states: Vec<usize>,
    pub init_states: Vec<usize>,
}

impl BuchiAutomata {
    pub fn new(num_state: usize) -> Self {
        Self {
            forward: vec![Vec::new(); num_state],
            backward: vec![Vec::new(); num_state],
            accepting_states: Vec::new(),
            init_states: Vec::new(),
        }
    }

    pub fn num_state(&self) -> usize {
        self.forward.len()
    }

    pub fn add_edge(&mut self, from: usize, to: usize, label: DdNode) {
        assert!(from < self.num_state());
        assert!(to < self.num_state());
        self.forward[from].push((to, label.clone()));
        self.backward[to].push((from, label));
    }

    pub fn add_init_state(&mut self, state: usize) {
        self.init_states.push(state);
    }

    pub fn add_accepting_state(&mut self, state: usize) {
        self.accepting_states.push(state);
    }
}

pub fn counter(cudd: &mut Cudd) -> BuchiAutomata {
    let mut automata = BuchiAutomata::new(3);
    let bit0 = cudd.ith_var(0);
    let bit1 = cudd.ith_var(1);
    automata.add_edge(0, 0, &bit0 | &bit1);
    automata.add_edge(0, 1, !&bit0 & !&bit1);
    automata.add_edge(1, 0, &bit0 ^ &bit1);
    automata.add_edge(1, 1, !&bit0 & !&bit1);
    automata.add_edge(1, 2, bit0 & bit1);
    automata.add_edge(2, 2, cudd.true_node());
    automata.add_init_state(0);
    automata.add_accepting_state(2);
    automata
}
