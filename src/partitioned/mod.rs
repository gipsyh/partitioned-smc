mod fair;
mod reachable;
mod statistic;
mod worker;

use self::statistic::Statistic;
use crate::{automata::BuchiAutomata, command::Args, ltl::ltl_to_automata_preprocess, BddManager};
use fsmbdd::FsmBdd;
use smv::{bdd::SmvBdd, Expr, Prefix, Smv};
use std::time::{Duration, Instant};
use sylvan::lace_run;

pub struct PartitionedSmc {
    manager: BddManager,
    fsmbdd: FsmBdd<BddManager>,
    automata: BuchiAutomata,
    args: Args,
    statistic: Statistic,
}

impl PartitionedSmc {
    pub fn new(
        manager: BddManager,
        fsmbdd: FsmBdd<BddManager>,
        automata: BuchiAutomata,
        args: Args,
    ) -> Self {
        Self {
            manager,
            fsmbdd,
            automata,
            args,
            statistic: Statistic::default(),
        }
    }

    pub fn check(&mut self) -> bool {
        let mut reach = vec![self.manager.constant(false); self.automata.num_state()];
        for init_state in self.automata.init_states.iter() {
            reach[*init_state] |= &self.fsmbdd.init;
        }
        let start = Instant::now();
        reach = if self.args.close_lace_optimize {
            self.post_reachable(&reach)
        } else {
            lace_run(|context| self.lace_post_reachable(context, &reach))
        };
        self.statistic.post_reachable_time += start.elapsed();
        let start = Instant::now();
        let fair_states = if self.args.close_lace_optimize {
            self.fair_states(&reach)
        } else {
            lace_run(|context| self.lace_fair_states(context, &reach))
        };
        self.statistic.fair_cycle_time += start.elapsed();
        for accept in self.automata.accepting_states.iter() {
            if &reach[*accept] & &fair_states[*accept] != self.manager.constant(false) {
                return false;
            }
        }
        true
    }
}

fn get_ltl(smv: &Smv, extend_trans: &[usize]) -> Expr {
    dbg!(&smv.trans.len());
    dbg!(extend_trans);
    let smv = smv.flatten_defines();
    let trans_ltl = extend_trans
        .iter()
        .fold(Expr::LitExpr(true), |fold, extend| {
            fold & Expr::PrefixExpr(Prefix::LtlGlobally, Box::new(smv.trans[*extend].clone()))
        });
    let mut fairness = Expr::LitExpr(true);
    for fair in smv.fairness.iter() {
        let fair = Expr::PrefixExpr(
            Prefix::LtlGlobally,
            Box::new(Expr::PrefixExpr(Prefix::LtlFinally, Box::new(fair.clone()))),
        );
        fairness = fairness & fair;
    }
    let ltl = smv.ltlspecs[0].clone();
    let ltl = !Expr::InfixExpr(
        smv::Infix::Imply,
        Box::new(trans_ltl & fairness),
        Box::new(ltl),
    );
    let ltl = ltl_to_automata_preprocess(&smv, ltl);
    println!("{}", ltl);
    ltl
}

pub fn check(manager: BddManager, smv: Smv, args: Args) -> (bool, Duration) {
    let smv_bdd = SmvBdd::new(&manager, &smv);
    let mut fsmbdd = smv_bdd.to_fsmbdd(args.trans_method.into());
    fsmbdd.justice.clear();
    let ba = BuchiAutomata::from_ltl(
        get_ltl(&smv, &args.ltl_extend_trans),
        &manager,
        &smv_bdd.symbols,
        &smv_bdd.defines,
    );
    let mut partitioned_smc = PartitionedSmc::new(manager.clone(), fsmbdd, ba, args);
    dbg!("partitioned smc start checking");
    let start = Instant::now();
    let res = partitioned_smc.check();
    let time = start.elapsed();
    dbg!(partitioned_smc.statistic);
    (res, time)
}
