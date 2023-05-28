use clap::{Parser, ValueEnum};
use fsmbdd::TransBddMethod;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum TransMethod {
    Monolithic,
    Partition,
}

impl Into<TransBddMethod> for TransMethod {
    fn into(self) -> TransBddMethod {
        match self {
            TransMethod::Monolithic => TransBddMethod::Monolithic,
            TransMethod::Partition => TransBddMethod::Partition,
        }
    }
}

#[derive(Parser, Debug)]
/// Partitioned Symbolic Model Checking
pub struct Args {
    /// trans partition method
    #[arg(short = 'm', long, value_enum, default_value_t = TransMethod::Partition)]
    pub trans_method: TransMethod,

    /// parallel
    #[arg(short, long, default_value_t = false)]
    pub parallel: bool,

    /// extend trans
    #[arg(short = 'e', long)]
    pub ltl_extend_trans: Vec<usize>,
}