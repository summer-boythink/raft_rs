use raft_rs::{
    logs::Logs,
    rafts::raft::{Config, Raft},
    rpc::HttpPeer,
    state_machine::MemStateMachine,
    storage::MemStorage,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    str::FromStr,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{Extension, Json, Path},
    handler::{get, post},
    http::StatusCode,
    response::IntoResponse,
    Router,
};
use clap::Parser;

#[derive(Serialize, Deserialize)]
struct CreateUser {
    username: String,
}

#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    username: String,
}
/// Simple program to configure the network
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Local network address
    #[arg(short, long, required = true)]
    local: String,

    /// Peer network addresses
    #[arg(short, long, value_parser, num_args = 1.., value_delimiter = ' ', required = true)]
    peer: Vec<String>,

    /// RPC timeout duration
    #[arg(short, long, default_value_t = 100)]
    rpc_timeout: u32,

    /// Heartbeat timeout duration
    #[arg(long, default_value_t = 300)]
    heartbeat_timeout: u32,

    /// Heartbeat interval duration
    #[arg(long, default_value_t = 100)]
    heartbeat_interval: u32,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    assert!(args.peer.len() >= 2, "at least 2 peers");

    let local = args.local;
    let peers = args.peer;
    let rpc_timeout = args.rpc_timeout;
    let heartbeat_timeout = args.heartbeat_timeout;
    let heartbeat_interval = args.heartbeat_interval;

    let addr = SocketAddr::from_str(&local).unwrap();

    let node_addr: Vec<String> = vec![local].into_iter().chain(peers.into_iter()).collect();
    node_addr.sort();

    let mut id: usize = 0;
    let mut peers = HashMap::new();

    for (i, addr) in node_addr.iter().enumerate() {
        if addr == &local {
            id = i;
        } else {
            peers.insert(i as u32, HttpPeer::new(addr.clone()));
        }
    }

    let state_machine = MemStateMachine::new();
    let config = Config {
        rpc_timeout,
        heartbeat_timeout,
        heartbeat_interval,
    };
    httpServer(
        Raft::new(
            id,
            Logs::new(MemStorage::new(), state_machine),
            peers,
            config,
        ),
        state_machine,
        addr,
    )
    .await;
}

async fn httpServer(config: Raft, state_machine: MemStateMachine, addr: SocketAddr) -> _ {
    let state = Arc::new(Mutex::new(User {
        id: 1337,
        username: "".to_string(),
    }));

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/append_entries", post(create_user));

    axum::serve(addr, app.into_make_service()).await.unwrap();
}

async fn create_user(
    Json(payload): Json<CreateUser>,
    Extension(state): Extension<Arc<Mutex<User>>>,
) -> impl IntoResponse {
    let mut user = state.lock().await;
    user.username = payload.username;

    (StatusCode::CREATED, Json(user.clone()))
}
