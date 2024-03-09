use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant};

use crate::logs::{LogEntry, Logs};
use crate::rpc::Peer;

#[derive(Clone)]
enum RaftStatus {
    Follower,
    Candidate,
    Leader,
}

pub struct Config {
    rpc_timeout: u32,
    heartbeat_timeout: Duration,
    heartbeat_interval: u32,
}

pub struct Raft {
    leader_id: Option<u32>,
    id: u32,
    current_term: Arc<Mutex<u32>>,
    voted_for: Arc<Mutex<Option<u32>>>,
    status: Arc<Mutex<RaftStatus>>,
    heartbeat_timeout: Duration,
    logs: Logs,
    peers: HashMap<u32, Box<dyn Peer>>,
    config: Config,
    commit_emitter: HashMap<u32, Vec<oneshot::Sender<()>>>,
}

impl Raft {
    pub fn new(id: u32, logs: Logs, peers: HashMap<u32, Box<dyn Peer>>, config: Config) -> Self {
        Self {
            leader_id: None,
            id,
            current_term: Arc::new(Mutex::new(0)),
            voted_for: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(RaftStatus::Follower)),
            heartbeat_timeout: config.heartbeat_timeout,
            logs,
            peers,
            config,
            commit_emitter: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct AppendEntriesArgs {
    // leader term
    term: u32,
    // leader id
    leader_id: u32,
    // The index of the log entry immediately preceding the new log entry
    prev_log_index: u32,
    // The term of the log entry immediately preceding the new log entry
    prev_log_term: u32,
    // If it is used as a heartbeat, the log entry is empty
    entries: Vec<LogEntry>,
    // Index of the leader's highest known log entries that have been submitted
    leader_commit: u32,
}

#[derive(Serialize, Deserialize)]
pub struct AppendEntriesReply {
    term: u32,
    success: bool,
}

#[derive(Serialize, Deserialize)]
pub struct RequestVoteArgs {
    term: u32,
    candidate_id: u32,
    last_log_index: u32,
    last_log_term: u32,
}

#[derive(Serialize, Deserialize)]
pub struct RequestVoteReply {
    term: u32,
    vote_granted: bool,
}
