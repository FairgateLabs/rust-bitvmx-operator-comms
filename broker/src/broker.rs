use bitvmx_broker::{
    broker_storage::BrokerStorage,
    channel::channel::{DualChannel, LocalChannel},
    rpc::{errors::BrokerError, sync_server::BrokerSync, tls_helper::Cert, BrokerConfig},
};
// use bitvmx_broker::broker_memstorage::MemStorage;
use crate::allow_handler::AllowHandler;
use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};
use storage_backend::{storage::Storage, storage_config::StorageConfig};
use tracing::info;

pub struct Broker {
    broker: BrokerSync,
    local_channel: LocalChannel<BrokerStorage>,
    // local_channel: LocalChannel<MemStorage>,
    cert: Cert,
    allow_list: AllowHandler,
}

impl Broker {
    pub fn new(
        broker_port: u16,
        addr: Option<IpAddr>,
        privk: &str, // DER format
        allow_list: AllowHandler,
    ) -> Result<Self, BrokerError> {
        let storage_path = format!("/tmp/broker_p2p_{}", broker_port);
        let config = StorageConfig::new(storage_path.clone(), None);
        let broker_backend =
            Storage::new(&config).map_err(|e| BrokerError::StorageError(e.to_string()))?;
        let broker_backend = Arc::new(Mutex::new(broker_backend));
        let broker_storage = Arc::new(Mutex::new(BrokerStorage::new(broker_backend)));
        // let broker_storage = Arc::new(Mutex::new(MemStorage::new()));

        let cert = Cert::new_with_privk(privk)?;
        let pubk_hash = cert.get_pubk_hash()?;
        let broker_config = BrokerConfig::new(broker_port, addr, pubk_hash.clone())?;
        let broker = BrokerSync::new(
            &broker_config,
            broker_storage.clone(),
            cert.clone(),
            allow_list.get_allow_list(),
        );
        let local_channel = LocalChannel::new(pubk_hash.clone(), broker_storage.clone());

        Ok(Self {
            broker,
            local_channel,
            cert,
            allow_list,
        })
    }

    pub fn put(
        &self,
        dest_port: u16,
        dest_ip: Option<IpAddr>,
        dest_pubk_hash: String,
        data: String,
    ) -> Result<(), BrokerError> {
        // It doesnt check address when sending data, only when receiving //TODO: is this ok?
        let server_config = BrokerConfig::new(dest_port, dest_ip, dest_pubk_hash.clone())?;
        let channel = DualChannel::new(
            &server_config,
            self.cert.clone(),
            self.allow_list.get_allow_list(),
        )?;
        channel.send(None, data.clone())?;
        info!("Send data {:?} to broker with id {}", data, dest_pubk_hash);
        Ok(())
    }

    pub fn get(&self) -> Result<Option<(String, String)>, BrokerError> {
        if let Some((data, id)) = self.local_channel.recv()? {
            info!("Received data {:?} from broker with id {}", data, id);
            Ok(Some((id, data)))
        } else {
            Ok(None)
        }
    }

    pub fn close(&mut self) {
        self.broker.close();
    }
}
