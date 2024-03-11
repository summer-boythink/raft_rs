use serde::{Deserialize, Serialize};

use crate::logs::LogEntry;


#[derive(Serialize, Deserialize)]
pub struct AppendEntriesArgs {
    // leader term
    pub term: u32,
    // leader id
    pub leader_id: u32,
    // The index of the log entry immediately preceding the new log entry
    pub prev_log_index: u32,
    // The term of the log entry immediately preceding the new log entry
    pub prev_log_term: u32,
    // If it is used as a heartbeat, the log entry is empty
    pub entries: Vec<LogEntry>,
    // Index of the leader's highest known log entries that have been submitted
    pub leader_commit: u32,
}

#[derive(Serialize, Deserialize)]
pub struct AppendEntriesReply {
    pub term: u32,
    pub success: bool,
}

#[derive(Serialize, Deserialize)]
pub struct RequestVoteArgs {
    pub term: u32,
    pub candidate_id: u32,
    pub last_log_index: u32,
    pub last_log_term: u32,
}

#[derive(Serialize, Deserialize,Clone, Copy)]
pub struct RequestVoteReply {
    pub term: u32,
    pub vote_granted: bool,
}
