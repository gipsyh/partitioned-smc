use super::slice::Slice;
use crate::Bdd;

pub enum Message {
    Data(Bdd, usize, Slice),
    GC(Bdd),
    Quit,
}
