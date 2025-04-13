use std::{
    collections::{HashMap, VecDeque},
    fmt::Display,
    str::FromStr,
    sync::{Arc, Mutex},
};

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;
//pub type PeerId = u64;
pub type P2PAddress = String;
pub type LocalAllowList = String;

//feature in memmory
lazy_static! {
    pub static ref PEER_ID_MAP: Arc<Mutex<HashMap<PeerId, VecDeque<(PeerId, Vec<u8>)>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

fn get(my_id: PeerId) -> Option<(PeerId, Vec<u8>)> {
    let mut map = PEER_ID_MAP.lock().unwrap();
    if let Some(value) = map.get_mut(&my_id) {
        Some(value.pop_front()?)
    } else {
        None
    }
}

fn put(my_id: PeerId, peer_id: PeerId, data: Vec<u8>) {
    let mut map = PEER_ID_MAP.lock().unwrap();
    let entry = map.entry(peer_id).or_insert_with(VecDeque::new);
    entry.push_back((my_id, data));
}

#[derive(Clone, Hash, Default, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PeerId(String);

impl PeerId {
    pub fn to_base58(&self) -> String {
        // Simulate base58 encoding
        //format!("{}", self.0)
        self.0.clone()
    }
}

impl Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PeerId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        //let id = s.parse::<u64>()?;
        Ok(PeerId(s.to_string()))
    }
}

#[derive(Clone, Debug)]
pub struct Keypair {
    pub public_key: String,
    pub private_key: String,
}

impl Keypair {
    pub fn from_protobuf_encoding(data: &[u8]) -> Result<Self, String> {
        // Simulate decoding from protobuf
        if data.is_empty() {
            return Err("Invalid data".to_string());
        }

        let mut reverse = data.to_vec();
        reverse.reverse();
        Ok(Keypair {
            public_key: format!("{:?}", data).to_string(),
            private_key: format!("{:?}", reverse).to_string(),
        })
    }
}

pub struct P2pHandler {
    peer_id: PeerId,
    address: P2PAddress,
}

#[derive(Error, Debug, PartialEq)]
pub enum P2pHandlerError {
    #[error("Internal error")]
    Error,
}

#[derive(Debug, PartialEq)]
pub enum ReceiveHandlerChannel {
    Msg(PeerId, Vec<u8>),
    Error(P2pHandlerError),
}

impl P2pHandler {
    pub fn new<T: Default>(addr: String, key: Keypair) -> Result<Self, P2pHandlerError> {
        Ok(P2pHandler {
            peer_id: PeerId(key.public_key),
            address: addr,
        })
    }

    pub fn get_peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }

    pub fn get_address(&self) -> P2PAddress {
        self.address.clone()
    }

    pub fn check_receive(&mut self) -> Option<ReceiveHandlerChannel> {
        match get(self.peer_id.clone()) {
            Some((peer_id, data)) => {
                info!("Receive data from peer: {}: {:?}", peer_id, data);
                Some(ReceiveHandlerChannel::Msg(peer_id, data))
            }
            None => None,
        }
    }

    pub fn request(
        &self,
        peer_id: PeerId,
        address: P2PAddress,
        data: Vec<u8>,
    ) -> Result<(), String> {
        // Simulate sending a request to the peer

        info!(
            "Generate request to peer: {} add: {}: {:?}",
            peer_id, address, data
        );
        put(self.peer_id.clone(), peer_id, data);

        Ok(())
    }

    pub fn response(&self, peer_id: PeerId, data: Vec<u8>) -> Result<(), String> {
        // Simulate sending a request to the peer
        info!("Generate request to peer: {}  {:?}", peer_id, data);
        put(self.peer_id.clone(), peer_id, data);
        Ok(())
    }
}
