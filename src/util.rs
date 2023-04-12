use smv::{Expr, Prefix};

pub fn trans_expr_to_ltl_rec(expr: &Expr) -> Expr {
    match expr {
        Expr::PrefixExpr(prefix, expr) => match prefix {
            Prefix::Not => Expr::PrefixExpr(prefix.clone(), Box::new(trans_expr_to_ltl_rec(expr))),
            Prefix::Next => {
                Expr::PrefixExpr(Prefix::LtlNext, Box::new(trans_expr_to_ltl_rec(expr)))
            }
            _ => todo!(),
        },
        Expr::Ident(_) | Expr::LitExpr(_) => expr.clone(),
        Expr::CaseExpr(_) => todo!(),
        Expr::InfixExpr(infix, left, right) => Expr::InfixExpr(
            infix.clone(),
            Box::new(trans_expr_to_ltl_rec(&left)),
            Box::new(trans_expr_to_ltl_rec(&right)),
        ),
    }
}

pub fn trans_expr_to_ltl(expr: &Expr) -> Expr {
    let ltl = trans_expr_to_ltl_rec(expr);
    Expr::PrefixExpr(Prefix::LtlGlobally, Box::new(ltl))
}
