use std::{net::SocketAddr, str::FromStr, sync::{Arc, Mutex}};
use raft_rs::{raft::raft::Raft, rpc::HttpPeer, state_machine::MemStateMachine};
use serde::{Deserialize, Serialize};

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

fn main() {
    let args = Args::parse();

    assert!(args.peer.len() >= 2, "at least 2 peers");

    let local =  args.local;
    let peers =  args.peer;
    let rpc_timeout =  args.rpc_timeout;
    let heartbeat_timeout = args.heartbeat_timeout;
    let heartbeat_interval = args.heartbeat_interval;

    let addr = SocketAddr::from_str(&local).unwrap();

    let node_addr: Vec<String> = vec![local].into_iter().chain(peers.into_iter()).collect();
    node_addr.sort();

    let mut id: usize = 0;
    let mut peers = Vec::new();
    
    for (i, addr) in node_addr.iter().enumerate() {
        if addr == &local {
            id = i;
        } else {
            peers.push(HttpPeer::new(addr.clone()));
        }
    }

    let stateMachine = MemStateMachine::new();

    httpServer(
        Raft::new(id, logs, peers, config)
        stateMachine,
        addr
    );

    let state = Arc::new(Mutex::new(User {
        id: 1337,
        username: "".to_string(),
    }));

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/users", post(create_user))
        .layer(axum::AddExtensionLayer::new(state));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn httpServer() -> _ {
    todo!()
}

async fn create_user(
    Json(payload): Json<CreateUser>,
    Extension(state): Extension<Arc<Mutex<User>>>,
) -> impl IntoResponse {
    let mut user = state.lock().await;
    user.username = payload.username;

    (StatusCode::CREATED, Json(user.clone()))
}
