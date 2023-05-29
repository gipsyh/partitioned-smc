#![feature(stmt_expr_attributes)]

mod automata;
mod cav00;
mod command;
mod ltl;
mod partitioned;
mod util;

use crate::partitioned::PartitionedSmc;
use automata::BuchiAutomata;
use clap::Parser;
use smv::{bdd::SmvBdd, Smv};

type BddManager = cudd::Cudd;
type Bdd = cudd::Bdd;

fn main() {
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/mutex/mutex-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/short/short-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/ring/ring-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/examples/counter/2bit/counter-flat.smv").unwrap();

    let input_file =

    // LMCS2006
    // "../MC-Benchmark/partitioned-smc/abp4-flat-p2.smv";
    // "../MC-Benchmark/partitioned-smc/abp8-flat-p2.smv";
    // "../MC-Benchmark/partitioned-smc/prod-cons-flat-p0.smv";
    // "../MC-Benchmark/partitioned-smc/production-cell-flat-p1.smv";

    // let smv =
    //     Smv::from_file("../MC-Benchmark/NuSMV-2.6-examples/example_cmu/dme1-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/LMCS-2006/dme/dme3-flat.smv").unwrap();

    // HWMCC08
    // "../MC-Benchmark/partitioned-smc/viscoherencep1-flat.smv";
    // "../MC-Benchmark/partitioned-smc/viscoherencep2-flat.smv";
    // "../MC-Benchmark/partitioned-smc/viscoherencep5-flat.smv";

    // HWMCC17
    // "../MC-Benchmark/partitioned-smc/cunim1ro-flat.smv";
    // "../MC-Benchmark/hwmcc17/single/bj08amba2g1-flat.smv";

    // HWMCC19
    "../MC-Benchmark/hwmcc19/single/aig/goel/industry/cal9/cal9-flat.smv";

    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/arbi0s08bugp03-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cutarb8ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cutf3ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cuhanoi7ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cuhanoi10ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cujc12ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/cunim1ro-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/live/arbixs08bugp03-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/single/shift1add262144-flat.smv").unwrap();
    // let smv = Smv::from_file("../MC-Benchmark/hwmcc17/single/ringp0-flat.smv").unwrap();
    // let smv = Smv::from_file(").unwrap();

    let args = command::Args::parse();
    let smv = Smv::from_file(input_file).unwrap();
    // smv.flatten_defines();
    let manager = BddManager::new();
    let smv_bdd = SmvBdd::new(&manager, &smv, &[]);
    let fsmbdd = smv_bdd.to_fsmbdd(args.trans_method.into());
    let ba = BuchiAutomata::from_ltl(
        ltl::get_ltl(&smv, &args.ltl_extend_trans),
        &manager,
        &smv_bdd.symbols,
        &smv_bdd.defines,
    );

    let time = partitioned::check(manager, fsmbdd, ba, args.parallel);
    println!("{:?}", time);
}
