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

    pub fn last_index(&self) -> u32 {
        self.store.last_index()
    }

    pub fn last(&self) -> LogEntry {
        self.store.entry(self.store.last_index()).unwrap()
    }

    pub fn entry(&self, log_index: u32) -> Option<LogEntry> {
        self.store.entry(log_index)
    }

    pub fn append(&mut self, command: String, term: u32) -> u32 {
        let log_index = self.last_index() + 1;
        self.store.truncate_and_append(vec![LogEntry {
            log_term: term,
            log_index,
            command,
        }]);
        log_index
    }

    pub fn append_entries(
        &mut self,
        prev_log_index: u32,
        prev_log_term: u32,
        mut entries: Vec<LogEntry>,
        leader_commit: u32,
    ) -> bool {
        let entry = self.entry(prev_log_index);
        if entry.map_or(false, |e| e.log_term != prev_log_term) {
            return false;
        }

        if !entries.is_empty() {
            entries = entries.split_off(
                (self.find_conflict(&entries) - entries[0].log_index)
                    .try_into()
                    .unwrap(),
            );
            self.store.truncate_and_append(entries);
        }

        if leader_commit > self.commit_index() {
            *self.commit_index.lock().unwrap() =
                leader_commit.min(prev_log_index + entries.len() as u32);
        }
        true
    }

    pub fn batch_entries(&self, start_log_index: u32, max_len: usize) -> (u32, u32, Vec<LogEntry>) {
        let mut entries = self
            .store
            .batch_entries(start_log_index - 1, (max_len + 1).try_into().unwrap());
        let prev_log = if entries.is_empty() {
            self.last()
        } else {
            entries.remove(0)
        };

        (prev_log.log_index, prev_log.log_term, entries)
    }

    pub fn is_up_to_date(&self, log_index: u32, log_term: u32) -> bool {
        let last = self.last();
        log_term > last.log_term || (last.log_term == log_term && log_index >= last.log_index)
    }

    pub fn find_conflict(&self, entries: &[LogEntry]) -> u32 {
        for e in entries {
            let entry = self.entry(e.log_index);
            if entry.map_or(true, |entry| entry.log_term != e.log_term) {
                return e.log_index;
            }
        }
        entries[0].log_index
    }
}
