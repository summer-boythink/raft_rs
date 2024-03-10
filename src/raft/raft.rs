use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::{select, task, time};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant};

use crate::logs::{LogEntry, Logs};
use crate::rpc::Peer;

use super::timeout::ResettableTimeout;

#[derive(Clone, PartialEq)]
enum RaftStatus {
    Follower,
    Candidate,
    Leader,
}

pub struct Config {
    pub rpc_timeout: u32,
    pub heartbeat_timeout: Duration,
    pub heartbeat_interval: u32,
}

pub struct Raft {
    leader_id: Option<u32>,
    id: u32,
    current_term: Arc<Mutex<u32>>,
    voted_for: Arc<Mutex<Option<u32>>>,
    status: Arc<Mutex<RaftStatus>>,
    heartbeat_timeout: ResettableTimeout,
    logs: Logs,
    peers: HashMap<u32, Box<dyn Peer>>,
    config: Config,
    commit_emitter: HashMap<u32, Vec<oneshot::Sender<()>>>,
    shutdown: UnboundedSender<bool>,
    receive_shutdown: UnboundedReceiver<bool>,
    receive_heartbeat: UnboundedReceiver<bool>,
}

impl Raft {
    pub fn new(id: u32, logs: Logs, peers: HashMap<u32, Box<dyn Peer>>, config: Config) -> Self {
        let (shutdown_send, shutdown_recv) = mpsc::unbounded_channel();
        let (emit_heartbeat, receive_heartbeat) = mpsc::unbounded_channel();
        Self {
            leader_id: None,
            id,
            current_term: Arc::new(Mutex::new(0)),
            voted_for: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(RaftStatus::Follower)),
            heartbeat_timeout: ResettableTimeout::new(
                config.heartbeat_timeout,
                emit_heartbeat.clone(),
            ),
            logs,
            peers,
            config,
            commit_emitter: HashMap::new(),
            shutdown: shutdown_send,
            receive_shutdown: shutdown_recv,
            receive_heartbeat,
        }
    }

    pub fn start(&mut self) {
        self.run();
        self.heartbeat_timeout.start();
    }

    async fn run(&mut self) {
        let shutdown = self.receive_shutdown.recv();
        tokio::select! {
            _ = shutdown => {
                return;
            }
            _ = time::sleep(Duration::from_secs(1)) => {
                self.leader_id = None;
                match *(self.status.clone().lock().unwrap()) {
                    RaftStatus::Follower => self.run_follower().await,
                    RaftStatus::Candidate => todo!(),
                    RaftStatus::Leader => todo!(),
                }
            }
        }
    }

    async fn run_follower(&mut self) {
        log::info!(
            "entering follower state. id: {} term: {}",
            self.id,
            self.current_term.lock().unwrap()
        );
        self.heartbeat_timeout.reset().await;
        while *(self.status.clone().lock().unwrap()) == RaftStatus::Follower {
            let r = select! {
                _ = self.receive_heartbeat.recv() => "heartbeatTimeout",
                _ = self.receive_shutdown.recv() => "shutdown",
            };
            match r {
                "heartbeatTimeout" => *self.status.lock().unwrap() = RaftStatus::Candidate,
                "shutdown" => return,
                _ => (),
            }
        }
    }

    async fn run_candidate(&mut self) {
        log::info!(
            "entering candidate state. id: {} term: {}",
            self.id,
            self.current_term.lock().unwrap()
        );
        while *self.status.lock().unwrap() == RaftStatus::Candidate {
            let r = select! {
                _ = self.elect_self() => "electSelf",
                _ = self.receive_shutdown() => "shutdown",
            };
            match r {
                "electSelf" => {
                    // 如果接收到大多数服务器的选票，那么就变成领导人
                    let mut granted_votes = 1;
                    let votes_needed = self.quorum_size();
                    for vote in r.value {
                        if vote.term > self.current_term {
                            log::warn!(
                                "[runCandidate] newer term discovered, fallback to follower",
                            );
                            *self.status.lock().unwrap() = RaftStatus::Follower;
                            self.current_term = vote.term;
                            return;
                        }
                        if vote.vote_granted {
                            granted_votes += 1;
                        }
                        if granted_votes >= votes_needed {
                            log::info!("election won. tally: {}", granted_votes);
                            *self.status.lock().unwrap() = RaftStatus::Leader;
                            return;
                        }
                    }
                    // 竞选失败
                    time::sleep(self.config.heartbeat_interval).await;
                }
                "shutdown" => return,
                _ => (),
            }
        }
    }
    async fn elect_self(&mut self) -> Vec<RequestVoteReply> {
        // 在转变成候选人后就立即开始选举过程
        // 自增当前的任期号（currentTerm）
        // 给自己投票
        let mut current_term = self.current_term.lock().unwrap();
        *current_term += 1;
        let mut voted_for = self.voted_for.lock().unwrap();
        *voted_for = Some(self.id);
        drop(current_term);
        drop(voted_for);

        let last_log_entry = self.logs.last();
        let vote_replies: Vec<_> = join_all(self.peer_ids().iter().map(|&peer_id| {
            let args = RequestVoteArgs {
                term: *self.current_term.lock().unwrap(),
                candidate_id: self.id,
                last_log_index: last_log_entry.log_index,
                last_log_term: last_log_entry.log_term,
            };
            self.peers[&peer_id].request_vote(args)
        }))
        .await;
        vote_replies.into_iter().filter_map(Result::ok).collect()
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
