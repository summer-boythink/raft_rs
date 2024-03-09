use log::info;
use rmp_serde::Deserializer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum Command {
    Rm { key: String },
    Set { key: String, value: String },
}

pub trait StateMachine {
    fn apply(&mut self, command: Vec<u8>);
}

pub struct MemStateMachine {
    pub state: HashMap<String, String>,
}

impl MemStateMachine {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.state.get(key)
    }
}

impl StateMachine for MemStateMachine {
    fn apply(&mut self, command: Vec<u8>) {
        let mut de = Deserializer::new(&command[..]);
        let cmd: Command = Deserialize::deserialize(&mut de).unwrap();
        info!("{:?}", cmd);
        match cmd {
            Command::Set { key, value } => {
                self.state.insert(key, value);
            }
            Command::Rm { key } => {
                self.state.remove(&key);
            }
        }
    }
}
