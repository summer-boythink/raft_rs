use super::raft::{AppendEntriesArgs, AppendEntriesReply, RequestVoteArgs, RequestVoteReply};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};
use std::time::Duration;
use tokio::time::timeout;

pub trait Peer {
    // 追加条目
    fn append_entries(
        &self,
        aea: AppendEntriesArgs,
        timeout: Duration,
    ) -> tokio::task::JoinHandle<Result<AppendEntriesReply, reqwest::Error>>;

    // 请求投票
    fn request_vote(
        &self,
        rv: RequestVoteArgs,
        timeout: Duration,
    ) -> tokio::task::JoinHandle<Result<RequestVoteReply, reqwest::Error>>;
}

pub struct HttpPeer {
    addr: String,
    client: Client,
}

impl HttpPeer {
    fn new(addr: String) -> Self {
        Self {
            addr,
            client: Client::new(),
        }
    }

    async fn post<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        req: &T,
    ) -> Result<R, reqwest::Error> {
        let resp = self
            .client
            .post(&format!("{}/{}", self.addr, method))
            .json(req)
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }
}

impl Peer for HttpPeer {
    fn append_entries(
        &self,
        aea: AppendEntriesArgs,
        t: Duration,
    ) -> tokio::task::JoinHandle<Result<AppendEntriesReply, reqwest::Error>> {
        let fut = self.post("append_entries", &aea);
        let timed_fut = timeout(t, fut);
        tokio::spawn(async move {
            timed_fut
                .await
                .map_err(|_| Error::new(ErrorKind::TimedOut, "operation timed out"))
                .unwrap()
        })
    }

    fn request_vote(
        &self,
        rv: RequestVoteArgs,
        t: Duration,
    ) -> tokio::task::JoinHandle<Result<RequestVoteReply, reqwest::Error>> {
        let fut = self.post("request_vote", &rv);
        let timed_fut = timeout(t, fut);
        tokio::spawn(async move {
            timed_fut
                .await
                .map_err(|_| Error::new(ErrorKind::TimedOut, "operation timed out"))
                .unwrap()
        })
    }
}
