use crate::{automata::BuchiAutomata, Bdd, BddManager};
use fsmbdd::FsmBdd;
use std::{
    mem::forget,
    ops::AddAssign,
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
                Message::Data(_, a) => {
                    panic!();
                }
                Message::GC(_) => todo!(),
                Message::Quit => (),
            }
        }
        *self.active.lock().unwrap() = 0;
    }

    pub fn init(&mut self, forward: bool, init: Bdd) {
        let mut init = self.manager.translocate(&init);
        if !forward && init != self.manager.constant(false) {
            init = self.fsmbdd.pre_image(&init);
        }
        let ba_trans = if forward {
            self.forward.clone()
        } else {
            self.backward.clone()
        };
        let update = self.propagate(&ba_trans, init);
        self.active.lock().unwrap().add_assign(update);
    }

    pub fn start(&mut self, forward: bool, constraint: Option<Bdd>) -> Bdd {
        let mut reach = self.manager.constant(false);
        let constraint = constraint.map(|bdd| self.manager.translocate(&bdd));
        let ba_trans = if forward {
            self.forward.clone()
        } else {
            self.backward.clone()
        };
        loop {
            let mut update = self.manager.constant(false);
            let mut num_update: i32 = 0;
            match self.receiver.recv().unwrap() {
                Message::Data(data, src) => {
                    num_update -= 1;
                    update |= self.manager.translocate(&data);
                    forget(data);
                }
                Message::GC(_) => todo!(),
                Message::Quit => {
                    return reach;
                }
            }
            while let Ok(message) = self.receiver.try_recv() {
                match message {
                    Message::Data(data, src) => {
                        num_update -= 1;
                        update |= self.manager.translocate(&data);
                        forget(data);
                    }
                    Message::GC(_) => todo!(),
                    Message::Quit => todo!(),
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
                num_update += self.propagate(&ba_trans, update) as i32;
            }
            let mut active = self.active.lock().unwrap();
            // dbg!(*active);
            // dbg!(num_update);
            active.add_assign(num_update);
            let active_value = *active;
            // dbg!(active_value);
            drop(active);
            if active_value == 0 {
                self.quit();
                return reach;
            }
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
