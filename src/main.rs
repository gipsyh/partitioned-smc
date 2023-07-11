mod automata;
mod command;
mod ltl;
mod partitioned;
mod traditional;
mod util;

use clap::Parser;
use command::Algorithm;
use smv::Smv;

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
    // "abp8-flat-p0.smv";
    // "prod-cons-flat-p1.smv";
    // "production-cell-flat-p1.smv";

    // HWMCC08
    // "viscoherencep1-flat.smv"; // 1 4
    // "viscoherencep2-flat.smv";
    // "viscoherencep5-flat.smv";

    // HWMCC17
    "cunim1ro-flat.smv"; // 0 1 3
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
    let input_file = format!("./benchmark/{}", input_file);
    let args = command::Args::parse();
    let smv = Smv::from_file(input_file).unwrap();
    let manager = BddManager::init(args.parallel);
    let algorithm = match args.algorithm {
        Algorithm::Partitioned => partitioned::check,
        Algorithm::Traditional => traditional::check,
    };
    let (res, time) = algorithm(manager, smv, args);
    println!("res: {}, time: {:?}", res, time);
}
