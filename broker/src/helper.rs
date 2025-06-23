use serde::{Deserialize, Serialize};
use std::{fmt::Display, fs, net::IpAddr, str::FromStr};

pub struct AddressParser;

impl AddressParser {
    pub fn parse(addr: &str) -> Result<(IpAddr, u16), String> {
        let parts: Vec<&str> = addr.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() != 4 || (parts[0] != "ip4" && parts[0] != "ip6") || parts[2] != "tcp" {
            return Err("Invalid address format".to_string());
        }
        let ip = IpAddr::from_str(parts[1]).map_err(|e| format!("Invalid IP: {}", e))?;
        let port = parts[3]
            .parse::<u16>()
            .map_err(|e| format!("Invalid port: {}", e))?;
        Ok((ip, port))
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct PeerInfo {
    peer_id: PeerId,
    broker_id: u16,
    address: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PeerInfoYaml {
    key: String,
    broker_id: u16,
    address: String,
}

pub struct PeerMapper {
    peers: Vec<PeerInfo>,
}

impl PeerMapper {
    pub fn new(path: &str) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| format!("Cannot read file: {}", e))?;
        let yaml_peers: Vec<PeerInfoYaml> =
            serde_yaml::from_str(&content).map_err(|e| format!("YAML is invalid: {}", e))?;

        let peers: Vec<PeerInfo> = yaml_peers
            .into_iter()
            .map(|yaml| -> Result<PeerInfo, String> {
                let key_bytes =
                    hex::decode(&yaml.key).map_err(|e| format!("Failed to decode key: {}", e))?;
                let keypair = Keypair::from_protobuf_encoding(&key_bytes)
                    .map_err(|e| format!("Failed to create keypair: {}", e))?;
                Ok(PeerInfo {
                    peer_id: PeerId(keypair.public_key),
                    broker_id: yaml.broker_id,
                    address: yaml.address,
                })
            })
            .collect::<Result<_, _>>()?;

        Ok(PeerMapper { peers })
    }

    pub fn peer_id_to_broker_id(&self, peer_id: PeerId) -> Result<u16, String> {
        self.peers
            .iter()
            .find(|p| p.peer_id == peer_id)
            .map(|p| p.broker_id)
            .ok_or_else(|| format!("Broker ID not found for Peer ID: {}", peer_id))
    }

    pub fn broker_id_to_peer_id(&self, broker_id: u16) -> Result<PeerId, String> {
        self.peers
            .iter()
            .find(|p| p.broker_id == broker_id)
            .map(|p| p.peer_id.clone())
            .ok_or_else(|| format!("Peer ID not found for Broker ID: {}", broker_id))
    }

    pub fn peer_id_to_address(&self, peer_id: PeerId) -> Result<&str, String> {
        self.peers
            .iter()
            .find(|p| p.peer_id == peer_id)
            .map(|p| p.address.as_str())
            .ok_or_else(|| format!("Address not found for Peer ID: {}", peer_id))
    }
}

#[derive(Clone, Hash, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PeerId(pub String);

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
