use futures::Future;
use std::time::Duration;

pub trait Peer {
    // 追加条目
    fn append_entries(
        &self,
        aea: AppendEntriesArgs,
        timeout: Duration,
    ) -> Box<dyn Future<Output = Result<AppendEntriesReply, Box<dyn std::error::Error>>>>;

    // 请求投票
    fn request_vote(
        &self,
        rv: RequestVoteArgs,
        timeout: Duration,
    ) -> Box<dyn Future<Output = Result<RequestVoteReply, Box<dyn std::error::Error>>>>;
}

pub struct HttpPeer {
    addr: String,
}

impl HttpPeer {
    pub fn new(addr: String) -> Self {
        Self { addr }
    }
}
