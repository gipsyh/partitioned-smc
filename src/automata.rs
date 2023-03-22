use cudd::{Cudd, DdNode};

pub struct BuchiAutomata {
    pub transitions: Vec<Vec<usize>>,
    pub label: Vec<DdNode>,
    pub accepting_states: Vec<usize>,
}

impl BuchiAutomata {
    pub fn new() -> Self {
        Self {
            transitions: Vec::new(),
            label: Vec::new(),
            accepting_states: Vec::new(),
        }
    }

    pub fn num_state(&self) -> usize {
        self.label.len()
    }

    pub fn add_state(&mut self, bdd: DdNode) -> usize {
        self.label.push(bdd);
        self.transitions.push(Vec::new());
        self.label.len() - 1
    }

    pub fn add_edge(&mut self, from: usize, to: usize) {
        assert!(from < self.label.len());
        assert!(to < self.label.len());
        self.transitions[from].push(to);
    }

    pub fn add_accepting_state(&mut self, state: usize) {
        self.accepting_states.push(state);
    }
}

pub fn counter(cudd: &mut Cudd) -> BuchiAutomata {
    let mut automata = BuchiAutomata::new();
    let bit0 = cudd.ith_var(0);
    let bit1 = cudd.ith_var(1);
    automata.add_state(&bit0 | &bit1);
    automata.add_state(!&bit0 & !&bit1);
    automata.add_state(bit0 & bit1);
    automata.add_edge(0, 0);
    automata.add_edge(0, 1);
    automata.add_edge(1, 0);
    automata.add_edge(1, 1);
    automata.add_edge(1, 2);
    automata.add_edge(2, 2);
    automata.add_accepting_state(2);
    automata
}
