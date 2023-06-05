use crate::{automata::BuchiAutomata, command::Args, ltl::ltl_to_automata_preprocess, BddManager};
use smv::{bdd::SmvBdd, Smv};
use std::time::{Duration, Instant};

pub fn check(manager: BddManager, smv: Smv, args: Args) -> (bool, Duration) {
    let smvbdd = SmvBdd::new(&manager, &smv);
    let fsmbdd = smvbdd.to_fsmbdd(args.trans_method.into());
    let ltl = ltl_to_automata_preprocess(&smv, !smv.ltlspecs[0].clone());
    let ltl_fsmbdd =
        BuchiAutomata::from_ltl(ltl, &manager, &smvbdd.symbols, &smvbdd.defines).to_fsmbdd();
    let product = fsmbdd.product(&ltl_fsmbdd);

    let start = Instant::now();
    let forward = product.reachable_from_init();
    let fair_cycle = product.fair_cycle_with_constrain(&forward);
    ((fair_cycle & forward).is_constant(false), start.elapsed())
}
