use smv::{Expr, Prefix, Smv};

pub fn trans_expr_to_ltl_rec(expr: &Expr) -> Expr {
    match expr {
        Expr::PrefixExpr(prefix, expr) => match prefix {
            Prefix::Next => {
                Expr::PrefixExpr(Prefix::LtlNext, Box::new(trans_expr_to_ltl_rec(expr)))
            }
            _ => Expr::PrefixExpr(prefix.clone(), Box::new(trans_expr_to_ltl_rec(expr))),
        },
        Expr::Ident(_) => expr.clone(),
        Expr::LitExpr(_) => expr.clone(),
        Expr::CaseExpr(_) => todo!(),
        Expr::InfixExpr(infix, left, right) => Expr::InfixExpr(
            infix.clone(),
            Box::new(trans_expr_to_ltl_rec(&left)),
            Box::new(trans_expr_to_ltl_rec(&right)),
        ),
    }
}

pub fn trans_expr_to_ltl(expr: &Expr) -> Expr {
    trans_expr_to_ltl_rec(expr)
}

pub fn check_prositional(expr: &Expr, next: bool) -> bool {
    match expr {
        Expr::Ident(_) | Expr::LitExpr(_) => true,
        Expr::PrefixExpr(prefix, sub_expr) => match prefix {
            Prefix::Not => check_prositional(&sub_expr, next),
            Prefix::LtlNext => {
                if next {
                    check_prositional(&sub_expr, false)
                } else {
                    false
                }
            }
            Prefix::Next => {
                todo!()
            }
            _ => false,
        },
        Expr::InfixExpr(infix, left, right) => match infix {
            smv::Infix::Iff | smv::Infix::And | smv::Infix::Or | smv::Infix::Imply => {
                check_prositional(left, next) && check_prositional(right, next)
            }
            smv::Infix::LtlSince | smv::Infix::LtlUntil | smv::Infix::LtlRelease => false,
        },
        Expr::CaseExpr(_) => todo!(),
    }
}

pub fn ltl_next_to_next(expr: &mut Expr) {
    match expr {
        Expr::PrefixExpr(prefix, expr) => {
            if let Prefix::LtlNext = prefix {
                *prefix = Prefix::Next
            }
            ltl_next_to_next(expr)
        }
        Expr::InfixExpr(_, left, right) => {
            ltl_next_to_next(left);
            ltl_next_to_next(right);
        }
        Expr::CaseExpr(_) => todo!(),
        Expr::Ident(_) | Expr::LitExpr(_) => (),
    }
}

pub fn check_trans(expr: &Expr) -> Option<Expr> {
    match expr {
        Expr::PrefixExpr(prefix, sub) if matches!(prefix, Prefix::LtlGlobally) => {
            if check_prositional(sub, true) {
                let mut res = *sub.clone();
                ltl_next_to_next(&mut res);
                Some(res)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn ltl_classify(expr: Expr) -> (Vec<Expr>, Vec<Expr>, Vec<Expr>) {
    let ands = expr.partition_to_ands();
    assert!(ands.len() > 1);
    let mut inits = Vec::new();
    let mut trans = Vec::new();
    let mut ltls = Vec::new();
    for element in ands {
        if check_prositional(&element, false) {
            inits.push(element);
        } else if let Some(tran) = check_trans(&element) {
            trans.push(tran)
        } else {
            ltls.push(element)
        }
    }
    (inits, trans, ltls)
}

pub fn smv_ltl_classify(mut smv: Smv) -> Smv {
    assert!(smv.ltlspecs.len() == 1);
    let mut ltl = smv.ltlspecs[0].clone();
    if let Expr::PrefixExpr(Prefix::Not, expr) = ltl {
        ltl = *expr;
    } else {
        panic!();
    }
    let (mut inits, mut trans, ltls) = ltl_classify(ltl);
    smv.inits.append(&mut inits);
    smv.trans.append(&mut trans);
    smv.ltlspecs[0] = Expr::LitExpr(true);
    for ltl in ltls {
        smv.ltlspecs[0] = smv.ltlspecs[0].clone() & ltl;
    }
    smv.ltlspecs[0] = !smv.ltlspecs[0].clone();
    smv
}
