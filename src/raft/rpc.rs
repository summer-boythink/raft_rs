use super::raft::{AppendEntriesArgs, AppendEntriesReply, RequestVoteArgs, RequestVoteReply};
use reqwest::Client;
use serde::{Deserialize, Serialize};
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

impl Peer for HttpPeer {
    fn append_entries(
        &self,
        aea: AppendEntriesArgs,
        t: Duration,
    ) -> tokio::task::JoinHandle<Result<AppendEntriesReply, reqwest::Error>> {
        let addr = self.addr.clone();
        let client = self.client.clone();
        tokio::spawn(async move {
            let fut = HttpPeer::post(&addr, &client, "request_vote", &aea);
            let r = timeout(t, fut).await;
            r.unwrap()
        })
    }

    fn request_vote(
        &self,
        rv: RequestVoteArgs,
        t: Duration,
    ) -> tokio::task::JoinHandle<Result<RequestVoteReply, reqwest::Error>> {
        let addr = self.addr.clone();
        let client = self.client.clone();
        tokio::spawn(async move {
            let fut = HttpPeer::post(&addr, &client, "request_vote", &rv);
            let r = timeout(t, fut).await;
            r.unwrap()
        })
    }
}

impl HttpPeer {
    async fn post<T: Serialize, R: for<'de> Deserialize<'de>>(
        addr: &str,
        client: &Client,
        method: &str,
        req: &T,
    ) -> Result<R, reqwest::Error> {
        let resp = client
            .post(&format!("{}/{}", addr, method))
            .json(req)
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }
}
