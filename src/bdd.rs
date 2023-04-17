use aig::{Aig, AigNodeId};
use cudd::{Cudd, Bdd};

fn _rec_get_trans_bdd(aig: &Aig, node: AigNodeId, cudd: &mut Cudd) -> Bdd {
    if aig.nodes[node].is_latch_input() {
        let index = aig
            .latchs
            .iter()
            .enumerate()
            .find(|(_, x)| x.input == node)
            .map(|(idx, _)| idx)
            .unwrap();
        return cudd.ith_var(index);
    }
    let fanin0 = aig.nodes[node].fanin0();
    let mut fanin0_bdd = _rec_get_trans_bdd(aig, fanin0.node_id(), cudd);
    if fanin0.compl() {
        fanin0_bdd = !fanin0_bdd;
    }
    let fanin1 = aig.nodes[node].fanin1();
    let mut fanin1_bdd = _rec_get_trans_bdd(aig, fanin1.node_id(), cudd);
    if fanin1.compl() {
        fanin1_bdd = !fanin1_bdd;
    }
    fanin0_bdd & fanin1_bdd
}

pub fn _get_trans_bdd(aig: &Aig, cudd: &mut Cudd) -> Bdd {
    let mut bdd = cudd.constant(true);
    for i in 0..aig.latchs.len() {
        let mut next_bdd = _rec_get_trans_bdd(aig, aig.latchs[i].next.node_id(), cudd);
        if aig.latchs[i].next.compl() {
            next_bdd = !next_bdd;
        }
        let next_var_bdd = cudd.ith_var(i + aig.latchs.len());
        bdd &= !(next_bdd ^ next_var_bdd);
    }
    bdd
}
