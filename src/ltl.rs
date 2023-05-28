use crate::util::trans_expr_to_ltl;
use smv::{Expr, Prefix, Smv};

pub fn get_ltl(smv: &Smv, extend_trans: &[usize]) -> Expr {
    dbg!(&smv.trans.len());
    dbg!(extend_trans);
    // let smv = smv.flatten_defines();
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
    let ltl = smv.flatten_to_propositional_define(&ltl);
    let ltl = smv.flatten_case(ltl);
    let ltl = trans_expr_to_ltl(&ltl);
    println!("{}", ltl);
    ltl
}
