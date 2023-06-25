use crate::{Bdd, BddManager};
use std::{
    ops::{Deref, DerefMut},
    sync::RwLock,
};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Slice {
    vars: Vec<(usize, bool)>,
}

impl Deref for Slice {
    type Target = Vec<(usize, bool)>;

    fn deref(&self) -> &Self::Target {
        &self.vars
    }
}

impl DerefMut for Slice {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vars
    }
}

impl Slice {
    pub fn new<I: IntoIterator<Item = (usize, bool)>>(vars: I) -> Self {
        Self {
            vars: vars.into_iter().collect(),
        }
    }

    pub fn bdd(&self, manager: &BddManager) -> Bdd {
        manager.cube(self.vars.iter().copied())
    }
}

pub struct SliceManager {
    _max_slice: usize,
    slices: RwLock<Vec<Slice>>,
}

impl SliceManager {
    pub fn new(slice: Vec<Slice>, max_slice: usize) -> Self {
        Self {
            _max_slice: max_slice,
            slices: RwLock::new(slice),
        }
    }

    pub fn get_slices(&self) -> Vec<Slice> {
        self.slices.read().unwrap().clone()
    }

    pub fn _add_slice(&self, _id: usize, _var: usize) -> bool {
        todo!()
    }
}
