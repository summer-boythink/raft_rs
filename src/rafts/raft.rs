use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::{join, select, task, time};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant};

use crate::rafts::logs::{LogEntry, Logs};
use crate::rafts::rpc::Peer;
use super::args::{AppendEntriesArgs, AppendEntriesReply, RequestVoteArgs, RequestVoteReply};

use super::timeout::ResettableTimeout;
use super::error::Result;

#[derive(Clone, PartialEq)]
enum RaftStatus {
    Follower,
    Candidate,
    Leader,
}

pub struct Config {
    pub rpc_timeout: Duration,
    pub heartbeat_timeout: Duration,
    pub heartbeat_interval: Duration,
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
    receive_shutdown: Option<UnboundedReceiver<bool>>,
    receive_heartbeat: Option<UnboundedReceiver<bool>>,
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
            receive_shutdown: Some(shutdown_recv),
            receive_heartbeat:Some(receive_heartbeat),
        }
    }

    pub fn start(&mut self) {
        self.run();
        self.heartbeat_timeout.start();
    }

    async fn run(&mut self) {
        let mut shutdown = self.receive_shutdown.take().unwrap();
        tokio::select! {
            _ = shutdown.recv() => {
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
            let mut receive_shutdown = self.receive_shutdown.take().unwrap();
            let mut receive_heartbeat = self.receive_heartbeat.take().unwrap();

            let r = select! {
                _ = receive_heartbeat.recv() => "heartbeatTimeout",
                _ = receive_shutdown.recv() => "shutdown",
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
            let mut shutdown = self.receive_shutdown.take().unwrap();
            
            let mut elect_result:Vec<RequestVoteReply> = vec![];
            let r = select! {
                elect_res = self.elect() => {
                    elect_result = elect_res;
                    "electSelf"
                },
                _ = shutdown.recv() => "shutdown",
            };
            match r {
                "electSelf" => {
                    // 如果接收到大多数服务器的选票，那么就变成领导人
                    let mut granted_votes = 1;
                    let votes_needed = self.quorum_size();
                    for vote in elect_result {
                        if vote.term > *self.current_term.lock().unwrap() {
                            log::warn!(
                                "[runCandidate] newer term discovered, fallback to follower",
                            );
                            *self.status.lock().unwrap() = RaftStatus::Follower;
                            *self.current_term.lock().unwrap() = vote.term;
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


    pub async fn elect(&mut self) -> Vec<RequestVoteReply> {
        let mut current_term = self.current_term.lock().unwrap();
        *current_term += 1;
        let mut voted_for = self.voted_for.lock().unwrap();
        *voted_for = Some(self.id);
        drop(current_term);
        drop(voted_for);

        let last_log_entry = self.logs.last();
        let vote_replies:JoinHandle<Result<RequestVoteReply>> = join!(self.peer_ids().iter().map(|&peer_id| async {
            let args = RequestVoteArgs {
                term: *self.current_term.lock().unwrap(),
                candidate_id: self.id,
                last_log_index: last_log_entry.log_index,
                last_log_term: last_log_entry.log_term,
            };
            self.peers.get(&peer_id).unwrap().request_vote(args,self.config.rpc_timeout).await
        }));

        vote_replies.await.into_iter().filter_map(Result::ok).collect()
    }

    fn majority_index(&self, match_index: &HashMap<u64, u64>) -> u64 {
        let mut arr: Vec<_> = match_index.values().cloned().collect();
        arr.sort();
        arr[arr.len() / 2]
    }

    fn peer_ids(&self) -> Vec<u64> {
        self.peers.keys().cloned().collect()
    }

    fn quorum_size(&self) -> usize {
        self.peers.len() / 2 + 1
    }
}
