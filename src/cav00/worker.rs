use super::{
    other::Message,
    slice::{Slice, SliceManager},
};
use crate::{Bdd, BddManager};
use fsmbdd::FsmBdd;
use std::{
    cmp::max,
    ops::{AddAssign, SubAssign},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};

pub struct Worker {
    id: usize,
    manager: BddManager,
    fsmbdd: FsmBdd<BddManager>,
    slice: Option<Slice>,
    slice_manager: Arc<SliceManager>,
    sender: Vec<Sender<Message>>,
    receiver: Receiver<Message>,
    active: Arc<Mutex<i32>>,
}

impl Worker {
    fn quit(&mut self) {
        for i in 0..self.sender.len() {
            if i != self.id {
                self.sender[i].send(Message::Quit).unwrap();
            }
        }
    }

    fn slice_check(&self, state: &Bdd) {
        if let Some(slice) = &self.slice {
            assert!((state & !slice.bdd(&self.manager)).is_constant(false))
        } else {
            assert!(state.is_constant(false))
        }
    }

    fn propagate(&mut self, state: &Bdd) {
        if state.is_constant(false) {
            return;
        }
        let slices = self.slice_manager.get_slices();
        for i in 0..slices.len() {
            let slice_state = state & slices[i].bdd(&self.manager);
            if !slice_state.is_constant(false) {
                self.active.lock().unwrap().add_assign(1);
                self.sender[i]
                    .send(Message::Data(slice_state, self.id, slices[i].clone()))
                    .unwrap();
            }
        }
    }

    pub fn reset(&mut self, slice: Option<Slice>) {
        while let Ok(message) = self.receiver.try_recv() {
            match message {
                Message::Quit => (),
                _ => todo!(),
            }
        }
        let mut active = self.active.lock().unwrap();
        *active = max(self.id as i32 + 1, *active);
        self.slice = slice;
    }

    pub fn reachable(&mut self, from: Bdd, forward: bool, constraint: Option<Bdd>) -> Bdd {
        let mut reach = self.manager.constant(false);
        let from = self.manager.translocate(&from);
        let constraint = constraint.map(|bdd| self.manager.translocate(&bdd));
        let iamge_computation = if forward {
            FsmBdd::<BddManager>::post_image
        } else {
            FsmBdd::<BddManager>::pre_image
        };
        self.slice_check(&from);
        if !from.is_constant(false) {
            let image = iamge_computation(&self.fsmbdd, &from);
            self.propagate(&image);
        }
        loop {
            let mut active = self.active.lock().unwrap();
            active.sub_assign(1);
            let active_value = *active;
            drop(active);
            if active_value == 0 {
                self.quit();
                return reach;
            }
            let mut update = self.manager.constant(false);
            match self.receiver.recv().unwrap() {
                Message::Data(data, src, slice) => {
                    assert!(self.slice.clone().unwrap() == slice);
                    update |= self.manager.translocate(&data);
                    self.active.lock().unwrap().add_assign(1);
                    self.sender[src].send(Message::GC(data)).unwrap();
                }
                Message::GC(bdd) => {
                    drop(bdd);
                }
                Message::Quit => {
                    return reach;
                }
            }
            let mut num_update: i32 = 0;
            while let Ok(message) = self.receiver.try_recv() {
                match message {
                    Message::Data(data, src, slice) => {
                        num_update -= 1;
                        assert!(self.slice.clone().unwrap() == slice);
                        update |= self.manager.translocate(&data);
                        self.active.lock().unwrap().add_assign(1);
                        self.sender[src].send(Message::GC(data)).unwrap();
                    }
                    Message::GC(bdd) => {
                        num_update -= 1;
                        drop(bdd);
                    }
                    Message::Quit => panic!(),
                }
            }
            update &= !&reach;
            if let Some(constraint) = &constraint {
                update &= constraint;
            }
            reach |= &update;
            if !update.is_constant(false) {
                let image = iamge_computation(&self.fsmbdd, &update);
                self.propagate(&image);
            }
            let mut active = self.active.lock().unwrap();
            assert!(*active > 0);
            active.add_assign(num_update);
            assert!(*active >= 0);
            drop(active);
        }
    }

    pub fn create_workers(
        fsmbdd: &FsmBdd<BddManager>,
        num_worker: usize,
        slice_manager: Arc<SliceManager>,
    ) -> Vec<Self> {
        let mut recievers = vec![];
        let mut senders = vec![];
        let mut workers = vec![];
        let active = Arc::new(Mutex::new(0));
        for _ in 0..num_worker {
            let (sender, receiver) = channel();
            recievers.push(receiver);
            senders.push(sender);
        }
        for (id, receiver) in recievers.into_iter().enumerate() {
            let fsmbdd = fsmbdd.clone_with_new_manager();
            workers.push(Self {
                id,
                manager: fsmbdd.manager.clone(),
                fsmbdd,
                sender: senders.clone(),
                receiver,
                active: active.clone(),
                slice: None,
                slice_manager: slice_manager.clone(),
            })
        }
        workers
    }
}
