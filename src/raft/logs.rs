use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::state_machine::StateMachine;
use crate::storage::Storage;

#[derive(Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub log_index: u32,
    pub log_term: u32,
    pub command: String,
}

pub struct Logs {
    store: Box<dyn Storage>,
    state_machine: Arc<Mutex<dyn StateMachine>>,
    commit_index: Arc<Mutex<u32>>,
    last_applied: Arc<Mutex<u32>>,
}

impl Logs {
    pub fn new(store: Box<dyn Storage>, state_machine: Arc<Mutex<dyn StateMachine>>) -> Self {
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
            let mut buf = Vec::new();
            let command = self
                .store
                .entry(*self.last_applied.lock().unwrap())
                .unwrap()
                .command;
            let serializer = Serializer::new(&mut buf);
            serializer.into_inner().write(command.as_bytes());
            self.state_machine.lock().unwrap().apply(buf);
        }
        *commit_index = index;
    }
}
