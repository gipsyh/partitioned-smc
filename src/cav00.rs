use crate::PartitionedSmc;
use fsmbdd::partition::PartitionedFsmBdd;
// impl PartitionedSmc {
//     pub fn check_ltl_cav00(&mut self) -> bool {
//         dbg!(self.fsmbdd.manager.num_var());
//         let ba_fsmbdd = self.automata.to_fsmbdd();
//         let product = self.fsmbdd.product(&ba_fsmbdd);
//         dbg!(product.manager.num_var());
//         let partitioned_product = PartitionedFsmBdd::new(&product, &[28, 29]);
//         let forward = partitioned_product.reachable_from_init();
//         let fair_cycle = partitioned_product.fair_cycle_with_constrain(&forward);
//         for i in 0..forward.len() {
//             if &fair_cycle[i] & &forward[i] != self.manager.constant(false) {
//                 return false;
//             }
//         }
//         true
//     }

//     pub fn check_ltl(&mut self) -> bool {
//         let ba_fsmbdd = self.automata.to_fsmbdd();
//         let product = self.fsmbdd.product(&ba_fsmbdd);
//         let forward = product.reachable_from_init();
//         let fair_cycle = product.fair_cycle_with_constrain(&forward);
//         fair_cycle & forward == self.manager.constant(false)
//     }
// }
