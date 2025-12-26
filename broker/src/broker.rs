use bitvmx_broker::{
    broker_storage::BrokerStorage,
    channel::channel::{DualChannel, LocalChannel},
    identification::{
        allow_list::AllowList,
        identifier::{Identifier, PubkHash},
        routing::RoutingTable,
    },
    rpc::{errors::BrokerError, sync_server::BrokerSync, tls_helper::Cert, BrokerConfig},
};
// use bitvmx_broker::broker_memstorage::MemStorage;
use std::{
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};
use storage_backend::{storage::Storage, storage_config::StorageConfig};
use tracing::info;

pub const COMMS_ID: u8 = 0; // Default ID for communication

pub struct Broker {
    broker: BrokerSync,
    local_channel: Option<LocalChannel<BrokerStorage>>,
    // local_channel: LocalChannel<MemStorage>,
    cert: Cert,
    address: SocketAddr,
    allow_list: Arc<Mutex<AllowList>>,
}

impl Broker {
    pub fn new(
        address: SocketAddr,
        privk: &str, //File with PEM format
        allow_list: Arc<Mutex<AllowList>>,
        routing: Arc<Mutex<RoutingTable>>,
        storage_path: Option<String>,
    ) -> Result<Self, BrokerError> {
        let storage_path = match storage_path {
            Some(path) => path,
            None => format!("/tmp/broker_comms_{}", address.port()),
        };
        let config = StorageConfig::new(storage_path.clone(), None);
        let broker_backend =
            Storage::new(&config).map_err(|e| BrokerError::StorageError(e.to_string()))?;
        let broker_backend = Arc::new(Mutex::new(broker_backend));
        let broker_storage = Arc::new(Mutex::new(BrokerStorage::new(broker_backend)));
        let cert = Cert::from_key_file(privk)?;
        let pubk_hash = cert.get_pubk_hash()?;
        let broker_config =
            BrokerConfig::new(address.port(), Some(address.ip()), pubk_hash.clone());
        let broker = BrokerSync::new(
            &broker_config,
            broker_storage.clone(),
            cert.clone(),
            allow_list.clone(),
            routing,
        )?;
        let local_channel = LocalChannel::new(
            Identifier {
                pubkey_hash: pubk_hash.clone(),
                id: COMMS_ID,
            },
            broker_storage.clone(),
        );

        Ok(Self {
            broker,
            local_channel: Some(local_channel),
            cert,
            address,
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
        // It doesnt check address when sending data, only when receiving
        let server_config = BrokerConfig::new(dest_port, dest_ip, dest_pubk_hash.clone());
        let channel = DualChannel::new(
            &server_config,
            self.cert.clone(),
            None,
            self.allow_list.clone(),
        )?;
        channel.send_server(data.clone())?;
        info!("Send data {:?} to broker with id {}", data, dest_pubk_hash);
        Ok(())
    }

    pub fn get(&self) -> Result<Option<(Identifier, String)>, BrokerError> {
        match &self.local_channel {
            Some(channel) => {
                if let Some((data, identifier)) = channel.recv()? {
                    info!(
                        "Received data {:?} from broker with id {}",
                        data, identifier
                    );
                    Ok(Some((identifier, data)))
                } else {
                    Ok(None)
                }
            }
            None => Err(BrokerError::ClosedChannel),
        }
    }

    pub fn get_pubk_hash(&self) -> Result<PubkHash, BrokerError> {
        let pubk_hash = self.cert.get_pubk_hash()?;
        Ok(pubk_hash)
    }

    pub fn get_address(&self) -> SocketAddr {
        self.address
    }

    pub fn close(&mut self) {
        self.broker.close();
        self.local_channel.take();
    }
}
