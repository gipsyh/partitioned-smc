use std::{sync::RwLock, ops::{Deref, DerefMut}};
use crate::{Bdd, BddManager};

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
    max_slice: usize,
    slices: RwLock<Vec<Slice>>,
}

impl SliceManager {
    pub fn new(slice: Vec<Slice>, max_slice: usize) -> Self {
        Self {
            max_slice,
            slices: RwLock::new(slice),
        }
    }

    pub fn reset_slice(&self, slice: Vec<Slice>) {
        *self.slices.write().unwrap() = slice
    }

    pub fn get_slices(&self) -> Vec<Slice> {
        self.slices.read().unwrap().clone()
    }

    pub fn add_slice(&self, id: usize, var: usize) -> bool {
        todo!()
    }
}
