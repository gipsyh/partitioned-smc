mod automata;
mod bdd;

use crate::{
    automata::counter,
    bdd::{get_trans_bdd, post_image},
};
use aig::Aig;
use cudd::{Cudd, DdNode};

fn main() {
    let aig = Aig::from_file("../test/counter.aag").unwrap();
    println!("{}", aig);
    let mut cudd = Cudd::new();
    let trans_bdd = get_trans_bdd(&aig, &mut cudd);
    let init_cube: Vec<DdNode> = aig.latchs.iter().map(|l| !cudd.ith_var(l.input)).collect();
    let init_bdd = cudd.cube_bdd(init_cube.iter());

    let automata = counter(&mut cudd);
    let mut reach = vec![cudd.false_node(); automata.num_state()];
    reach[0] = &init_bdd & &automata.label[0];
    reach[1] = &init_bdd & &automata.label[1];
    let mut frontier = reach.clone();
    loop {
        dbg!(&reach);
        let mut new_frontier = vec![cudd.false_node(); automata.num_state()];
        for i in 0..frontier.len() {
            let post_image = post_image(&mut cudd, &(&frontier[i] & &trans_bdd));
            for next_state in automata.transitions[i].iter() {
                let update = &post_image & &automata.label[*next_state] & !&reach[*next_state];
                new_frontier[*next_state] = &new_frontier[*next_state] | &update;
            }
        }
        dbg!(&new_frontier);
        if new_frontier.iter().all(|bdd| bdd.is_false()) {
            break;
        }
        for i in 0..new_frontier.len() {
            reach[i] = &reach[i] | &new_frontier[i];
            frontier[i] = new_frontier[i].clone();
        }
    }
    dbg!(reach[2].is_false());
}
