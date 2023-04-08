use crate::automata::BuchiAutomata;
use cudd::DdNode;
use smv::bdd::SmvTransBdd;
use std::{
    ops::{AddAssign, SubAssign},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};

pub struct Worker {
    pub trans: SmvTransBdd,
    forward_sender: Vec<(Sender<Option<DdNode>>, DdNode)>,
    forward_receiver: Receiver<Option<DdNode>>,
    backward_sender: Vec<(Sender<Option<DdNode>>, DdNode)>,
    backward_receiver: Receiver<Option<DdNode>>,
    forward_quit_signal: Sender<Option<DdNode>>,
    backward_quit_signal: Sender<Option<DdNode>>,
    active: Arc<Mutex<usize>>,
}

impl Worker {
    pub fn new(
        trans: SmvTransBdd,
        forward_sender: Vec<(Sender<Option<DdNode>>, DdNode)>,
        forward_receiver: Receiver<Option<DdNode>>,
        backward_sender: Vec<(Sender<Option<DdNode>>, DdNode)>,
        backward_receiver: Receiver<Option<DdNode>>,
        forward_quit_signal: Sender<Option<DdNode>>,
        backward_quit_signal: Sender<Option<DdNode>>,
        active: Arc<Mutex<usize>>,
    ) -> Self {
        Self {
            trans,
            forward_sender,
            forward_receiver,
            backward_sender,
            backward_receiver,
            forward_quit_signal,
            backward_quit_signal,
            active,
        }
    }

    pub fn pre_image(&self, bdd: &DdNode) -> DdNode {
        let bdd = self.trans.cudd.translocate(bdd);
        self.trans.pre_image(&bdd)
    }

    pub fn post_image(&self, bdd: &DdNode) -> DdNode {
        let bdd = self.trans.cudd.translocate(bdd);
        self.trans.post_image(&bdd)
    }

    pub fn start(&mut self, forward: bool, init: DdNode, constraint: Option<DdNode>) -> DdNode {
        let mut reach = self.trans.cudd.constant(false);
        let mut init = self.trans.cudd.translocate(&init);
        let constraint = constraint.map(|bdd| self.trans.cudd.translocate(&bdd));
        let (senders, receiver, quit) = if forward {
            (
                &self.forward_sender,
                &self.forward_receiver,
                &self.forward_quit_signal,
            )
        } else {
            (
                &self.backward_sender,
                &self.backward_receiver,
                &self.backward_quit_signal,
            )
        };
        if !forward && init != self.trans.cudd.constant(false) {
            init = self.pre_image(&init);
        }
        for (sender, label) in senders {
            let send = &init & label;
            if send != self.trans.cudd.constant(false) {
                sender.send(Some(send)).unwrap();
                self.active.lock().unwrap().add_assign(1);
            }
        }
        let mut first_quit = false;
        loop {
            let mut active = self.active.lock().unwrap();
            if *active == 1 {
                quit.send(None).unwrap();
                first_quit = true;
            }
            active.sub_assign(1);
            drop(active);
            match receiver.recv().unwrap() {
                Some(bdd) => {
                    let mut update = self.trans.cudd.translocate(&bdd);
                    while let Ok(bdd) = receiver.try_recv() {
                        self.active.lock().unwrap().sub_assign(1);
                        update |= self.trans.cudd.translocate(&bdd.unwrap());
                    }
                    if !forward {
                        update &= !&reach;
                        if let Some(constraint) = &constraint {
                            update &= constraint;
                        }
                        reach |= &update;
                    }
                    if update != self.trans.cudd.constant(false) {
                        let mut update = if forward {
                            self.post_image(&update)
                        } else {
                            self.pre_image(&update)
                        };
                        if forward {
                            update &= !&reach;
                            reach |= &update;
                        }
                        for (sender, label) in senders {
                            let send = &update & label;
                            if send != self.trans.cudd.constant(false) {
                                sender.send(Some(send)).unwrap();
                                self.active.lock().unwrap().add_assign(1);
                            }
                        }
                    }
                }
                None => {
                    self.active.lock().unwrap().add_assign(1);
                    if !first_quit {
                        quit.send(None).unwrap();
                    }
                    return reach;
                }
            }
        }
    }
}

impl Worker {
    pub fn create_workers(trans: &SmvTransBdd, automata: &BuchiAutomata) -> Vec<Self> {
        let mut forward_recievers = vec![];
        let mut backward_recievers = vec![];
        let mut forward_dest_senders = vec![];
        let mut backward_dest_senders = vec![];
        let mut forward_quit_signals = vec![];
        let mut backward_quit_signals = vec![];
        let mut workers = vec![];
        let active = Arc::new(Mutex::new(automata.num_state()));
        for _ in 0..automata.num_state() {
            let (sender, receiver) = channel();
            forward_dest_senders.push(sender.clone());
            forward_recievers.push(receiver);
            forward_quit_signals.push(sender);
            let (sender, receiver) = channel();
            backward_dest_senders.push(sender.clone());
            backward_recievers.push(receiver);
            backward_quit_signals.push(sender);
        }
        let last = forward_quit_signals.pop().unwrap();
        forward_quit_signals.insert(0, last);
        let last = backward_quit_signals.pop().unwrap();
        backward_quit_signals.insert(0, last);
        for (
            i,
            (((forward_receiver, backward_receiver), forward_quit_signal), backward_quit_signal),
        ) in forward_recievers
            .into_iter()
            .zip(backward_recievers.into_iter())
            .zip(forward_quit_signals.into_iter())
            .zip(backward_quit_signals.into_iter())
            .enumerate()
        {
            let mut forward_senders = vec![];
            let mut backward_senders = vec![];
            for (dest, label) in automata.forward[i].iter() {
                forward_senders.push((forward_dest_senders[*dest].clone(), label.clone()));
            }
            for (dest, label) in automata.backward[i].iter() {
                backward_senders.push((backward_dest_senders[*dest].clone(), label.clone()));
            }
            let trans = trans.clone_with_new_cudd();
            for (_, sender) in forward_senders.iter_mut() {
                *sender = trans.cudd.translocate(sender);
            }
            for (_, sender) in backward_senders.iter_mut() {
                *sender = trans.cudd.translocate(sender);
            }
            workers.push(Self::new(
                trans,
                forward_senders,
                forward_receiver,
                backward_senders,
                backward_receiver,
                forward_quit_signal,
                backward_quit_signal,
                active.clone(),
            ))
        }
        workers
    }
}
