use serde::{Deserialize, Serialize};
use tracing::debug;
use std::fs::File;
use std::io::BufReader;
use std::{fmt::Display, fs, net::IpAddr, str::FromStr};
use x509_parser::{parse_x509_certificate, pem::Pem};

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

        // Get the directory of the YAML file as p2p peer pem path is relative to the yaml file
        let yaml_dir = std::path::Path::new(path).parent().unwrap_or(std::path::Path::new("."));

        let peers: Vec<PeerInfo> = yaml_peers
            .into_iter()
            .map(|yaml| -> Result<PeerInfo, String> {
                let pem_path = yaml_dir.join(&yaml.key);
                debug!("Resolved p2p PEM path: {}", pem_path.display());
                let peer_id = PeerId::from_pem_file(pem_path.to_str().unwrap())?;
                Ok(PeerInfo {
                    peer_id,
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
    pub fn from_pem_file(path: &str) -> Result<Self, String> {
        let pem_file = File::open(path).map_err(|e| format!("Cannot read file: {}", e))?;
        // Parse the PEM block
        let (pem, _) = Pem::read(BufReader::new(pem_file)).map_err(|e| format!("PEM parse error: {:?}", e))?;
            // Parse the X.509 certificate
        let (_, cert) = parse_x509_certificate(&pem.contents)
        .map_err(|e| format!("X509 parse error: {:?}", e))?;
        // Extract the public key in DER format
        let pubkey_der = cert.public_key().raw.to_vec();
        Ok(PeerId::from_der(pubkey_der))
    }

    pub fn from_der(public_key_der: Vec<u8>) -> Self {
        PeerId(hex::encode(public_key_der))
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
