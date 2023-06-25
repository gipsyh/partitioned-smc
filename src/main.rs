#![feature(stmt_expr_attributes)]

mod automata;
mod cav00;
mod command;
mod ltl;
mod partitioned;
mod traditional;
mod util;

use clap::Parser;
use command::Algorithm;
use smv::Smv;

// type BddManager = cudd::Cudd;
// type Bdd = cudd::Bdd;
type BddManager = sylvan::Sylvan;
type Bdd = sylvan::Bdd;

fn main() {
    // TEST
    // "../MC-Benchmark/LMCS-2006/mutex/mutex-flat.smv";
    // "../MC-Benchmark/LMCS-2006/short/short-flat.smv";
    // "../MC-Benchmark/LMCS-2006/ring/ring-flat.smv";
    // "../MC-Benchmark/examples/counter/2bit/counter-flat.smv";

    let input_file =
    // LMCS2006
    // "abp8-flat-p1.smv";
    // "prod-cons-flat-p2.smv";
    // "production-cell-flat-p1.smv";

    // HWMCC08
    // "viscoherencep1-flat.smv"; // 1 3
    "viscoherencep2-flat.smv";
    // "viscoherencep5-flat.smv";

    // HWMCC17
    // "cunim1ro-flat.smv";
    // "cuhanoi7ro-flat.smv";
    // "cuhanoi10ro-flat.smv";
    // "cuabq2mfro-flat.smv";

    // "../MC-Benchmark/hwmcc17/single/bj08amba2g1-flat.smv";
    // "../MC-Benchmark/hwmcc17/live/cutf3ro-flat.smv";

    // HWMCC19
    // "../MC-Benchmark/hwmcc19/single/aig/goel/industry/cal9/cal9-flat.smv";

    // "../MC-Benchmark/hwmcc17/live/arbi0s08bugp03-flat.smv").unwrap();
    // "../MC-Benchmark/hwmcc17/live/cutarb8ro-flat.smv";
    // "../MC-Benchmark/hwmcc17/live/cujc12ro-flat.smv";
    // "../MC-Benchmark/hwmcc17/live/arbixs08bugp03-flat.smv").unwrap();
    // "../MC-Benchmark/hwmcc17/single/ringp0-flat.smv";
    let input_file = format!("../benchmark/{}", input_file);
    let args = command::Args::parse();
    let smv = Smv::from_file(input_file).unwrap();
    let manager = BddManager::new();
    let algorithm = match args.algorithm {
        Algorithm::Partitioned => partitioned::check,
        Algorithm::Traditional => traditional::check,
        Algorithm::Cav00 => cav00::check,
    };
    let (res, time) = algorithm(manager, smv, args);
    println!("res: {}, time: {:?}", res, time);
}
