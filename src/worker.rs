use cudd::{Cudd, DdNode};

pub struct Worker {
    cudd: Cudd,
    trans: DdNode,
}

impl Worker {
    pub fn new(trans: &DdNode) -> Self {
        let cudd = Cudd::new();
        let trans = cudd.translocate(trans);
        Self { cudd, trans }
    }

    pub fn pre_image(&mut self, bdd: &DdNode) -> DdNode {
        let bdd = self.cudd.translocate(bdd);
        let num_var = self.cudd.num_var() / 2;
        let bdd = self.cudd.swap_vars(
            &bdd,
            (0..num_var).map(|x| x * 2),
            (0..num_var).map(|x| x * 2 + 1),
        );
        let bdd = bdd & &self.trans;
        self.cudd
            .exist_abstract(&bdd, (0..num_var).map(|x| x * 2 + 1))
    }

    pub fn post_image(&mut self, bdd: &DdNode) -> DdNode {
        let bdd = self.cudd.translocate(bdd);
        let num_var = self.cudd.num_var() / 2;
        let bdd = bdd & &self.trans;
        let bdd = self.cudd.exist_abstract(&bdd, (0..num_var).map(|x| x * 2));
        self.cudd.swap_vars(
            &bdd,
            (0..num_var).map(|x| x * 2 + 1),
            (0..num_var).map(|x| x * 2),
        )
    }
}
