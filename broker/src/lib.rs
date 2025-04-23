use bitvmx_broker::{
    broker_storage::BrokerStorage,
    channel::channel::{DualChannel, LocalChannel},
    rpc::{errors::BrokerError, sync_server::BrokerSync, BrokerConfig},
};
// use bitvmx_broker::broker_memstorage::MemStorage;
use storage_backend::{error::StorageError, storage::Storage};

use std::{
    net::IpAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use thiserror::Error;
use tracing::{error, info};

mod helper;
use crate::helper::{AddressParser, PeerMapper};
pub use helper::{Keypair, PeerId};

pub type P2PAddress = String;
pub type LocalAllowList = String;

struct Broker {
    broker: BrokerSync,
    local_channel: LocalChannel<BrokerStorage>,
    // local_channel: LocalChannel<MemStorage>,
}
impl Broker {
    pub fn new(broker_port: u16, addr: Option<IpAddr>) -> Result<Self, StorageError> {
        let storage_path = format!("/tmp/broker_p2p_{}", broker_port);
        let broker_backend = Storage::new_with_path(&PathBuf::from(storage_path.clone()))?;
        let broker_backend = Arc::new(Mutex::new(broker_backend));
        let broker_storage = Arc::new(Mutex::new(BrokerStorage::new(broker_backend)));
        // let broker_storage = Arc::new(Mutex::new(MemStorage::new()));
        let broker_config = BrokerConfig::new(broker_port, addr);
        let broker = BrokerSync::new(&broker_config, broker_storage.clone());
        let local_channel = LocalChannel::new(0, broker_storage.clone());
        Ok(Self {
            broker,
            local_channel,
        })
    }

    pub fn put(
        broker_port: u16,
        addr: Option<IpAddr>,
        my_id: u32,
        data: String,
    ) -> Result<(), BrokerError> {
        let config = BrokerConfig::new(broker_port, addr);
        let channel = DualChannel::new(&config, my_id);
        channel.send(0, data.clone())?;
        info!("Send data {:?} to broker with id {}", data, my_id);
        Ok(())
    }

    pub fn get(&self) -> Result<Option<(u32, String)>, BrokerError> {
        //sleep(std::time::Duration::from_millis(200));
        if let Some((data, id)) = self.local_channel.recv()? {
            info!("Received data {:?} from broker with id {}", data, id);
            Ok(Some((id, data)))
        } else {
            //info!("No data received from broker");
            Ok(None)
        }
    }

    pub fn close(&mut self) {
        self.broker.close();
    }
}

pub struct P2pHandler {
    peer_id: PeerId,
    address: P2PAddress,
    broker: Broker,
    peer_mapper: PeerMapper,
}

#[derive(Error, Debug, PartialEq)]
pub enum P2pHandlerError {
    #[error("Internal error: {0}")]
    Error(String),

    #[error("Broker error: {0}")]
    BrokerError(String),
}

#[derive(Debug, PartialEq)]
pub enum ReceiveHandlerChannel {
    Msg(PeerId, Vec<u8>),
    Error(P2pHandlerError),
}

impl P2pHandler {
    pub fn new<T: Default>(addr: String, key: Keypair) -> Result<Self, P2pHandlerError> {
        let (ip, port) = AddressParser::parse(&addr)
            .map_err(|_| P2pHandlerError::Error("Invalid address".to_string()))?;
        let broker = Broker::new(port, Some(ip))
            .map_err(|e| P2pHandlerError::Error(format!("Failed to create broker: {}", e)))?;
        let base_path = env!("CARGO_MANIFEST_DIR"); // The directory with this crate's Cargo.toml
        let config_path = format!("{}/config/peers.yaml", base_path);
        let peer_mapper = PeerMapper::new(&config_path).map_err(P2pHandlerError::Error)?;
        Ok(P2pHandler {
            peer_id: PeerId(key.public_key.to_string()),
            address: addr,
            broker,
            peer_mapper,
        })
    }

    pub fn get_peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }

    pub fn get_address(&self) -> P2PAddress {
        self.address.clone()
    }

    pub fn check_receive(&mut self) -> Option<ReceiveHandlerChannel> {
        match self.internal_check_receive() {
            Ok(Some(receive)) => Some(receive),
            Ok(None) => None,
            Err(err) => {
                error!("{}", err);
                None
            }
        }
    }

    fn internal_check_receive(&mut self) -> Result<Option<ReceiveHandlerChannel>, P2pHandlerError> {
        match self
            .broker
            .get()
            .map_err(|e| P2pHandlerError::BrokerError(e.to_string()))?
        {
            Some((id, data)) => match self.peer_mapper.broker_id_to_peer_id(id as u16) {
                Ok(peer_id) => {
                    let data = serde_json::from_str::<Vec<u8>>(&data)
                        .map_err(|e| P2pHandlerError::Error(e.to_string()))?;
                    info!("Receive data from peer: {}: {:?}", peer_id, data);
                    Ok(Some(ReceiveHandlerChannel::Msg(peer_id, data)))
                }
                Err(err) => {
                    error!("{}", err);
                    Err(P2pHandlerError::Error(err.to_string()))
                }
            },
            None => Ok(None),
        }
    }

    pub fn request(
        &self,
        peer_id: PeerId,
        address: P2PAddress,
        data: Vec<u8>,
    ) -> Result<(), P2pHandlerError> {
        // Simulate sending a request to the peer
        info!(
            "Generate request to peer: {} add: {}: {:?}",
            peer_id, address, data
        );

        let (ip, port) = AddressParser::parse(&address).map_err(P2pHandlerError::Error)?;
        let _id = self
            .peer_mapper
            .peer_id_to_broker_id(peer_id)
            .map_err(P2pHandlerError::Error)?; // Just check if the peer_id is valid
        let my_id = self
            .peer_mapper
            .peer_id_to_broker_id(self.peer_id.clone())
            .map_err(P2pHandlerError::Error)?;
        let data =
            serde_json::to_string(&data).map_err(|e| P2pHandlerError::Error(e.to_string()))?;

        Broker::put(port, Some(ip), my_id as u32, data)
            .map_err(|e| P2pHandlerError::BrokerError(e.to_string()))?;

        Ok(())
    }

    pub fn response(&self, peer_id: PeerId, data: Vec<u8>) -> Result<(), P2pHandlerError> {
        // Simulate sending a response to the peer
        info!("Generate response to peer: {}  {:?}", peer_id, data);

        let addr = self
            .peer_mapper
            .peer_id_to_address(peer_id.clone())
            .map_err(P2pHandlerError::Error)?;
        let (ip, port) = AddressParser::parse(addr).map_err(P2pHandlerError::Error)?;
        let my_id = self
            .peer_mapper
            .peer_id_to_broker_id(self.peer_id.clone())
            .map_err(P2pHandlerError::Error)?;
        let data =
            serde_json::to_string(&data).map_err(|e| P2pHandlerError::Error(e.to_string()))?;

        Broker::put(port, Some(ip), my_id as u32, data)
            .map_err(|e| P2pHandlerError::BrokerError(e.to_string()))?;

        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), P2pHandlerError> {
        Ok(self.broker.close())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //use tracing_subscriber;

    #[test]
    fn test_boker() {
        // tracing_subscriber::fmt()
        //     .without_time()
        //     .with_target(false)
        //     .init();
        let send_id = 1;
        let data = "hello".to_string();
        let mut broker = Broker::new(10000, None).unwrap();
        Broker::put(10000, None, send_id, data.clone()).unwrap();
        let received_data = broker.get().unwrap();
        assert_eq!(received_data, Some((send_id, data)));
        broker.close();
    }

    #[test]
    fn test_request_response() {
        // tracing_subscriber::fmt()
        //     .without_time()
        //     .with_target(false)
        //     .init();

        // Hardcoded keys and addresses
        let key1= "080112408888ee9eac838c47e3a17f354f30477357a7f6b63a6ceabe70f821ae76bcd461f87ef7c8f92b9486d8c82e25738cdf643311c700e25e0d105eff81b497f5abeb".to_string();
        let key1 = Keypair::from_protobuf_encoding(&hex::decode(key1.as_bytes()).unwrap()).unwrap();
        let key2= "080112406e52d71640c7c226a09da3d4f4299eadd636cb375037e34fcbbe2c8e93577ea82550385621b3f807ba79dc93725f50bca4dc2eee215e95dc9ef863dcfcf4bc1b".to_string();
        let key2 = Keypair::from_protobuf_encoding(&hex::decode(key2.as_bytes()).unwrap()).unwrap();
        let addr1 = "/ip4/127.0.0.1/tcp/61180".to_string();
        let addr2 = "/ip4/127.0.0.1/tcp/61181".to_string();

        let mut peer1 = P2pHandler::new::<()>(addr1, key1).unwrap();
        let mut peer2 = P2pHandler::new::<()>(addr2, key2).unwrap();

        let peer1_id = peer1.get_peer_id();
        let peer2_id = peer2.get_peer_id();
        let _peer1_address = peer1.get_address();
        let peer2_address = peer2.get_address();

        let request_data = b"hello peer2".to_vec();
        let response_data = b"hello peer1".to_vec();

        // peer1 sends request to peer2
        peer1
            .request(
                peer2_id.clone(),
                peer2_address.clone(),
                request_data.clone(),
            )
            .unwrap();

        // peer2 receives the request
        match peer2.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, peer1_id);
                assert_eq!(data, request_data);

                // peer2 sends a response back to peer1
                peer2
                    .response(from_id.clone(), response_data.clone())
                    .unwrap();
            }
            _ => panic!("Peer2 expected to receive a message"),
        }

        // peer1 receives the response
        match peer1.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, peer2_id);
                assert_eq!(data, response_data);
            }
            _ => panic!("Peer1 expected to receive a response"),
        }

        // Close the brokers
        peer1.stop().unwrap();
        peer2.stop().unwrap();
    }

    #[test]
    fn test_concurrent_requests() {
        // tracing_subscriber::fmt()
        //     .without_time()
        //     .with_target(false)
        //     .init();

        // Hardcoded keys and addresses
        let key1= "0801124098a7c8db342852696d63dd2be8db3d46d16180fd10429f8bdcdc25e299e44223cb449d3daa2efcb84541670187d2163d76cfd0e8f1c3fdad34163882ee0f2240".to_string();
        let key1 = Keypair::from_protobuf_encoding(&hex::decode(key1.as_bytes()).unwrap()).unwrap();
        let key2= "08011240fe31227bdbfe5555501e9724e32cf705d1852fc02accdee893ee5c1fcac93f9da8c8dc95334fde96b323decafe3261b7e657ce62630ed1e54a34409e8915a73a".to_string();
        let key2 = Keypair::from_protobuf_encoding(&hex::decode(key2.as_bytes()).unwrap()).unwrap();
        let addr1 = "/ip4/127.0.0.1/tcp/61182".to_string();
        let addr2 = "/ip4/127.0.0.1/tcp/61183".to_string();

        let mut peer1 = P2pHandler::new::<()>(addr1, key1).unwrap();
        let mut peer2 = P2pHandler::new::<()>(addr2, key2).unwrap();
        let peer1_id = peer1.get_peer_id();
        let peer2_id = peer2.get_peer_id();

        let peer1_address = peer1.get_address();
        let peer2_address = peer2.get_address();

        let request_data_1 = b"hello peer2".to_vec();
        let response_data_1 = b"hello peer1".to_vec();

        let request_data_2 = b"ping".to_vec();
        let response_data_2 = b"pong".to_vec();

        // peer1 sends request to peer2
        peer1
            .request(
                peer2_id.clone(),
                peer2_address.clone(),
                request_data_1.clone(),
            )
            .unwrap();

        // peer2 sends request to peer1
        peer2
            .request(
                peer1_id.clone(),
                peer1_address.clone(),
                request_data_2.clone(),
            )
            .unwrap();

        match peer2.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, peer1_id);
                assert_eq!(data, request_data_1);

                // peer2 sends a response back to peer1
                peer2
                    .response(from_id.clone(), response_data_1.clone())
                    .unwrap();
            }
            _ => panic!("Peer2 expected to receive a message"),
        }

        //peer1 receives the request
        match peer1.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, peer2_id);
                assert_eq!(data, request_data_2);

                // peer1 sends a response back to peer2
                peer1
                    .response(from_id.clone(), response_data_2.clone())
                    .unwrap();
            }
            _ => panic!("Peer1 expected to receive a message"),
        }

        // peer1 receives the response
        match peer1.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, peer2_id);
                assert_eq!(data, response_data_1);
            }
            _ => panic!("Peer1 expected to receive a response"),
        }

        // peer2 receives the response
        match peer2.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, peer1_id);
                assert_eq!(data, response_data_2);
            }
            _ => panic!("Peer2 expected to receive a response"),
        }

        // Close the brokers
        peer1.stop().unwrap();
        peer2.stop().unwrap();
    }
}
