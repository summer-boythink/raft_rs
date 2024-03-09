use std::sync::{Arc, Mutex};

use rmp_serde::Serializer;

use crate::state_machine::StateMachine;
use crate::storage::Storage;

#[derive(Clone)]
pub struct LogEntry {
    log_index: u32,
    log_term: u32,
    command: String,
}

pub struct Logs {
    store: dyn Storage,
    state_machine: Arc<Mutex<dyn StateMachine>>,
    commit_index: Arc<Mutex<u32>>,
    last_applied: Arc<Mutex<u32>>,
}

impl Logs {
    pub fn new(store: dyn Storage, state_machine: Arc<Mutex<dyn StateMachine>>) -> Self {
        Self {
            store,
            state_machine,
            commit_index: Arc::new(Mutex::new(0)),
            last_applied: Arc::new(Mutex::new(0)),
        }
    }

    pub fn commit_index(&self) -> u32 {
        *self.commit_index.lock().unwrap()
    }

    pub fn set_commit_index(&self, index: u32) {
        let mut commit_index = self.commit_index.lock().unwrap();
        while *commit_index < *self.last_applied.lock().unwrap() {
            *self.last_applied.lock().unwrap() += 1;
            self.state_machine.lock().unwrap().apply(
                Serializer::new(
                    &self
                        .entry(*self.last_applied.lock().unwrap())
                        .unwrap()
                        .command,
                )
                .unwrap(),
            );
        }
        *commit_index = index;
    }
}
