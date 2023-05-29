use crate::{automata::BuchiAutomata, Bdd, BddManager};
use fsmbdd::FsmBdd;
use std::{
    cmp::max,
    ops::{AddAssign, SubAssign},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};

enum Message {
    Data(Bdd, usize),
    GC(Bdd),
    Quit,
}

pub struct Worker {
    id: usize,
    manager: BddManager,
    fsmbdd: FsmBdd<BddManager>,
    sender: Vec<Sender<Message>>,
    receiver: Receiver<Message>,
    active: Arc<Mutex<i32>>,
    forward: Vec<(usize, Bdd)>,
    backward: Vec<(usize, Bdd)>,
}

impl Worker {
    #[inline]
    fn propagate(&mut self, ba_trans: &Vec<(usize, Bdd)>, data: Bdd) -> i32 {
        let mut num_send = 0;
        if data == self.manager.constant(false) {
            return num_send;
        }
        for (next, label) in ba_trans {
            let message = &data & label;
            if !message.is_constant(false) {
                self.active.lock().unwrap().add_assign(1);
                self.sender[*next]
                    .send(Message::Data(message, self.id))
                    .unwrap();
                num_send += 1;
            }
        }
        num_send
    }

    fn quit(&mut self) {
        for i in 0..self.sender.len() {
            if i != self.id {
                self.sender[i].send(Message::Quit).unwrap();
            }
        }
    }

    pub fn reset(&mut self) {
        while let Ok(message) = self.receiver.try_recv() {
            match message {
                Message::Data(_, _) => todo!(),
                Message::GC(_) => todo!(),
                Message::Quit => (),
            }
        }
        let mut active = self.active.lock().unwrap();
        *active = max(self.id as i32 + 1, *active);
    }

    pub fn start(&mut self, forward: bool, init: Bdd, constraint: Option<Bdd>) -> Bdd {
        let mut reach = self.manager.constant(false);
        let mut init = self.manager.translocate(&init);
        let constraint = constraint.map(|bdd| self.manager.translocate(&bdd));
        let ba_trans = if forward {
            self.forward.clone()
        } else {
            self.backward.clone()
        };
        if !forward && init != self.manager.constant(false) {
            init = self.fsmbdd.pre_image(&init);
        }
        self.propagate(&ba_trans, init);
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
            let mut num_update: i32 = 0;
            match self.receiver.recv().unwrap() {
                Message::Data(data, src) => {
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
            while let Ok(message) = self.receiver.try_recv() {
                match message {
                    Message::Data(data, src) => {
                        update |= self.manager.translocate(&data);
                        num_update -= 1;
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
            if !forward {
                update &= !&reach;
                if let Some(constraint) = &constraint {
                    update &= constraint;
                }
                reach |= &update;
            }
            if !update.is_constant(false) {
                let mut update = if forward {
                    self.fsmbdd.post_image(&update)
                } else {
                    self.fsmbdd.pre_image(&update)
                };
                if forward {
                    update &= !&reach;
                    reach |= &update;
                }
                self.propagate(&ba_trans, update);
            }
            let mut active = self.active.lock().unwrap();

            assert!(*active > 0);
            active.add_assign(num_update);
            assert!(*active >= 0);

            drop(active);
        }
    }
}

impl Worker {
    pub fn create_workers(fsmbdd: &FsmBdd<BddManager>, automata: &BuchiAutomata) -> Vec<Self> {
        let mut recievers = vec![];
        let mut senders = vec![];
        let mut workers = vec![];
        let active = Arc::new(Mutex::new(0));
        for _ in 0..automata.num_state() {
            let (sender, receiver) = channel();
            recievers.push(receiver);
            senders.push(sender);
        }
        for (id, reciever) in recievers.into_iter().enumerate() {
            let fsmbdd = fsmbdd.clone_with_new_manager();
            let mut forward = automata.forward[id].clone();
            let mut backward = automata.backward[id].clone();
            for (_, bdd) in forward.iter_mut() {
                *bdd = fsmbdd.manager.translocate(bdd);
            }
            for (_, bdd) in backward.iter_mut() {
                *bdd = fsmbdd.manager.translocate(bdd);
            }
            workers.push(Self {
                id,
                manager: fsmbdd.manager.clone(),
                fsmbdd,
                sender: senders.clone(),
                receiver: reciever,
                active: active.clone(),
                forward,
                backward,
            })
        }
        workers
    }
}
