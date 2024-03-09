use std::sync::{Arc, Mutex};

use crate::logs::LogEntry;

pub trait Storage {
    fn last_index(&self) -> u32;
    fn entry(&self, index: u32) -> Option<LogEntry>;
    fn batch_entries(&self, start_index: u32, max_len: u32) -> Vec<LogEntry>;
    fn truncate_and_append(&mut self, entries: Vec<LogEntry>);
}

pub struct MemStorage {
    entries: Arc<Mutex<Vec<LogEntry>>>,
}

impl MemStorage {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(vec![LogEntry {
                log_index: 0,
                log_term: 0,
                command: "".to_string(),
            }])),
        }
    }
}

impl Storage for MemStorage {
    fn last_index(&self) -> u32 {
        let entries = self.entries.lock().unwrap();
        entries.last().unwrap().log_index
    }

    fn entry(&self, index: u32) -> Option<LogEntry> {
        let entries = self.entries.lock().unwrap();
        entries.get(index as usize).cloned()
    }

    fn batch_entries(&self, start_index: u32, max_len: u32) -> Vec<LogEntry> {
        let entries = self.entries.lock().unwrap();
        entries[start_index as usize..(start_index + max_len) as usize].to_vec()
    }

    fn truncate_and_append(&mut self, entries: Vec<LogEntry>) {
        if entries.is_empty() {
            return;
        }
        let mut storage_entries = self.entries.lock().unwrap();
        for e in entries {
            storage_entries[e.log_index as usize] = e;
        }
        let last_id = entries.last().unwrap().log_index;
        storage_entries.truncate((last_id + 1) as usize);
    }
}
