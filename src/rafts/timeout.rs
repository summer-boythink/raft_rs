use std::time::Duration;
use tokio::{
    sync::mpsc::UnboundedSender,
    time::{self, Instant},
};

pub struct ResettableTimeout {
    delay: Duration,
    timer: Option<Instant>,
    callback: UnboundedSender<bool>,
}

impl ResettableTimeout {
    pub fn new(delay: Duration, callback: UnboundedSender<bool>) -> Self {
        Self {
            delay,
            timer: None,
            callback,
        }
    }

    pub fn stop(&mut self) {
        self.timer = None;
    }

    pub async fn start(&mut self) {
        // todo: task::spawn
        let delay = self.delay.as_secs_f64() * (1.0 + rand::random::<f64>());
        self.timer = Some(time::Instant::now() + Duration::from_secs_f64(delay));
        let timer = self.timer.unwrap();
        time::sleep_until(timer).await;
        self.callback.send(true).unwrap();
    }

    pub async fn reset(&mut self) {
        self.stop();
        self.start().await;
    }
}
