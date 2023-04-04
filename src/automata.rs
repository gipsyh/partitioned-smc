use cudd::{Cudd, DdNode};
use logic_form::Expr;
use nom::{
    bytes::complete::{tag, take_until},
    character::complete::{line_ending, space0},
    multi::many1,
    sequence::{delimited, terminated},
    IResult,
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct BuchiAutomata {
    pub forward: Vec<Vec<(usize, DdNode)>>,
    pub backward: Vec<Vec<(usize, DdNode)>>,
    pub accepting_states: Vec<usize>,
    pub init_states: Vec<usize>,
}

impl BuchiAutomata {
    pub fn new() -> Self {
        Self {
            forward: Vec::new(),
            backward: Vec::new(),
            accepting_states: Vec::new(),
            init_states: Vec::new(),
        }
    }

    pub fn num_state(&self) -> usize {
        self.forward.len()
    }

    fn extend_to(&mut self, to: usize) {
        while self.forward.len() <= to {
            self.forward.push(Vec::new());
            self.backward.push(Vec::new());
        }
    }

    pub fn add_edge(&mut self, from: usize, to: usize, label: DdNode) {
        self.extend_to(from);
        self.extend_to(to);
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

fn skip_line(input: &str) -> IResult<&str, &str> {
    terminated(take_until("\n"), line_ending)(input)
}

fn parse_trans(input: &str) -> IResult<&str, (&str, &str)> {
    let (input, _) = delimited(space0, tag("::"), space0)(input)?;
    let (input, cond) = terminated(take_until(" -> goto "), tag(" -> goto "))(input)?;
    let (input, dist) = terminated(take_until("\n"), line_ending)(input)?;
    Ok((input, (cond, dist)))
}

fn parse_state(input: &str) -> IResult<&str, (&str, Vec<(&str, &str)>)> {
    let (input, ident) = terminated(take_until(":"), tag(":"))(input)?;
    let (input, _) = skip_line(input)?;
    let (input, _) = skip_line(input)?;
    let (input, trans) = many1(parse_trans)(input)?;
    let (input, _) = skip_line(input)?;
    Ok((input, (ident, trans)))
}

impl BuchiAutomata {
    fn state_ident_get_id<'a>(
        &mut self,
        map: &mut HashMap<&'a str, usize>,
        ident: &'a str,
    ) -> usize {
        map.get(ident)
            .map(|x| *x)
            .or_else(|| {
                assert!(map.insert(ident, self.num_state()).is_none());
                self.extend_to(self.num_state());
                map.get(ident).map(|x| *x)
            })
            .unwrap()
    }

    pub fn parse(input: &str, cudd: &mut Cudd, symbols: &HashMap<String, usize>) -> Self {
        let mut ret = Self::new();
        let mut state_map = HashMap::new();
        let (input, _) = skip_line(input).unwrap();
        let (input, states) = many1(parse_state)(input).unwrap();
        let (input, _) = skip_line(input).unwrap();
        assert!(input.is_empty());
        for (ident, trans) in states {
            let state_id = ret.state_ident_get_id(&mut state_map, ident);
            if ident.starts_with("accept_") {
                ret.add_accepting_state(state_id);
            }
            if ident.ends_with("_init") {
                ret.add_init_state(state_id);
            }
            let mut cond = cudd.constant(true);
            for i in 0..trans.len() {
                let edge = trans[i].0;
                let dist = trans[i].1;
                let dist = ret.state_ident_get_id(&mut state_map, dist);
                if edge == "(1)" {
                    continue;
                }
                let edge = Expr::from(edge);
                let edge_bdd = edge.to_bdd(cudd, symbols);
                cond = cond & !&edge_bdd;
                ret.add_edge(state_id, dist, edge_bdd);
            }
            for i in 0..trans.len() {
                let edge = trans[i].0;
                let dist = trans[i].1;
                let dist = ret.state_ident_get_id(&mut state_map, dist);
                if edge == "(1)" {
                    ret.add_edge(state_id, dist, cond.clone());
                }
            }
        }
        ret
    }
}

// pub fn counter(cudd: &mut Cudd) -> BuchiAutomata {
//     let mut automata = BuchiAutomata::new(3);
//     let bit0 = cudd.ith_var(0);
//     let bit1 = cudd.ith_var(1);
//     automata.add_edge(0, 0, &bit0 | &bit1);
//     automata.add_edge(0, 1, !&bit0 & !&bit1);
//     automata.add_edge(1, 0, &bit0 ^ &bit1);
//     automata.add_edge(1, 1, !&bit0 & !&bit1);
//     automata.add_edge(1, 2, bit0 & bit1);
//     automata.add_edge(2, 2, cudd.constant(true));
//     automata.add_init_state(0);
//     automata.add_accepting_state(2);
//     automata
// }
