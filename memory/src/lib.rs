use std::{
    collections::{HashMap, VecDeque},
    fmt::Display,
    net::SocketAddr,
    str::FromStr,
    sync::{Arc, Mutex},
};

use broker::{allow_handler::AllowHandler, p2p_handler::P2pHandler as AuxHandler};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;
//feature in memmory
lazy_static! {
    pub static ref PEER_ID_MAP: Arc<Mutex<HashMap<String, VecDeque<(String, Vec<u8>)>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

fn get(my_id: String) -> Option<(String, Vec<u8>)> {
    let mut map = PEER_ID_MAP.lock().unwrap();
    if let Some(value) = map.get_mut(&my_id) {
        Some(value.pop_front()?)
    } else {
        None
    }
}

fn put(my_id: String, pubk_hash: String, data: Vec<u8>) {
    let mut map = PEER_ID_MAP.lock().unwrap();
    let entry = map.entry(pubk_hash).or_insert_with(VecDeque::new);
    entry.push_back((my_id, data));
}

// #[derive(Clone, Hash, Default, Debug, Eq, PartialEq, Serialize, Deserialize)]
// impl String {
//     pub fn to_base58(&self) -> String {
//         // Simulate base58 encoding
//         //format!("{}", self.0)
//         self.0.clone()
//     }
// }

// impl Display for String {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", self.0)
//     }
// }

// impl FromStr for String {
//     type Err = std::num::ParseIntError;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         //let id = s.parse::<u64>()?;
//         Ok(String(s.to_string()))
//     }
// }
// #[derive(Clone, Debug)]
// pub struct Keypair {
//     pub public_key: String,
//     pub private_key: String,
// }

// impl Keypair {
//     pub fn from_protobuf_encoding(data: &[u8]) -> Result<Self, String> {
//         // Simulate decoding from protobuf
//         if data.is_empty() {
//             return Err("Invalid data".to_string());
//         }

//         let mut reverse = data.to_vec();
//         reverse.reverse();
//         Ok(Keypair {
//             public_key: format!("{:?}", data).to_string(),
//             private_key: format!("{:?}", reverse).to_string(),
//         })
//     }
// }

pub struct P2pHandler {
    pubk_hash: String,
    address: SocketAddr,
}

#[derive(Error, Debug, PartialEq)]
pub enum P2pHandlerError {
    #[error("Internal error")]
    Error,
}

#[derive(Debug, PartialEq)]
pub enum ReceiveHandlerChannel {
    Msg(String, Vec<u8>),
    Error(P2pHandlerError),
}

impl P2pHandler {
    pub fn new(
        address: SocketAddr,
        privk: &str, // DER format
        _allow_list: AllowHandler,
    ) -> Result<Self, P2pHandlerError> {
        Ok(P2pHandler {
            pubk_hash: AuxHandler::get_pubk_hash_from_privk(privk)
                .map_err(|_| P2pHandlerError::Error)?,
            address,
        })
    }

    pub fn check_receive(&mut self) -> Option<ReceiveHandlerChannel> {
        match get(self.pubk_hash.clone()) {
            Some((pubk_hash, data)) => {
                info!("Receive data from peer: {}: {:?}", pubk_hash, data);
                Some(ReceiveHandlerChannel::Msg(pubk_hash, data))
            }
            None => None,
        }
    }

    pub fn send(
        &self,
        pubk_hash: &str,
        address: SocketAddr,
        data: Vec<u8>,
    ) -> Result<(), P2pHandlerError> {
        info!(
            "Send data to peer: {} at address: {}: {:?}",
            pubk_hash, address, data
        );
        put(self.pubk_hash.clone(), pubk_hash.to_string(), data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;
    struct PeerInfo {
        privk: String,
        pubk_hash: String,
        address: SocketAddr,
    }
    impl PeerInfo {
        fn new(privk: &str, port: u16) -> Self {
            let pubk_hash = AuxHandler::get_pubk_hash_from_privk(privk).unwrap();
            let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
            let address = SocketAddr::new(ip, port);
            Self {
                privk: privk.to_string(),
                pubk_hash,
                address,
            }
        }
    }
    fn get_info(port1: u16, port2: u16) -> (PeerInfo, PeerInfo) {
        let privk1 = "308204be020100300d06092a864886f70d0101010500048204a8308204a40201000282010100a757eeb2bc74fed438885e29060ae22d8c3ae9542fcac76e798fb4ce500a345ad260cf85072046a15b8e7c84c60514a1abb9c3d66e3c5121aaf3a1e4c2ba1e70da3ad26f2e8eb34ed6c6110c1ad6942a7f3e911f5b5fa7491698d3a97808aae28ba272116a42b25ffabe79510ab508a878d38e5246cd5a25172a5071fc4113039c266c1d6df17486548e78bc235c8f8a6316ffef15a04f0909ad55538f902a6b7b182f33307998e917d9a19808c203c53247c5600bd2ebb323e8758413df610d33290d7a6358886a3a115aeecbe2d899bb39a787bbc007cd08a786ac8efa1ff1bc5b8cfe699f89699abacff34eba15df504ad7099e4001b4e754e3c2904358c102030100010282010031cfe6e9a5575e1365d091d6bc49b911bdd03b6c27ddc0878dffccde2ccd1cd07c16fd2ea7f45f91e0630585b03c0aec24e5e2f98d4ebf07ba8f52fd7949558e5a2770445023821451b21b98f2d434be81a9ea20df5e15b997d45e0cf002047bf2fca3dfb335af4b0aa470104393a7c41e533ae61ad53da414c52fb4fe559086e24a0ed9fdfc38d2932c0079bf45e53999020a6227e2fb482208a69ba972f8b5866cb3e96923d10c20575f6de854b8705e59c66d4f21cd92171de980c7731bba12edf8f2e3e66c7d86d92bf408e95a3b93975f1f56a7245f83c3b338c0ccb864e59aea8cebef73f2dc8191cf7b798dc6bed4fcd3e8316b70e619efaf3b3b3bd102818100ebdafe144dc671cbf4c99c4e0c5caec184d905b10066165e18b39709da16f764208926667699cdcff5671beac9cd3ce1cf20d2ef84c01cc3d0cdb7853059c8387ab6d21dbc91dc9b569e24062fe6ade43a6639003ff0b01ff876acacc7f7cde243106a37058a1a11e431589a91786acdcfd27705093c103366fb6df0c8dd674702818100b5a2eaf6d9036537abbe182465375ad5625e0ec397493139c0b9ccedcbbf6783745aeeaf0e006a0ae42592ad82f34c0b595c5209c81f2fe33e1ba05af22b1ce22d075c78fc51d205c023f7e73bb2a344f01f093cec36b5ea5f0b04c60964260d30c0c44bf3899983e79c552937c1f0c5da99ebe53b3ce6b6f8413d0230a9d3b702818100ae778561c1929d1531537de32233e135d7aeae0e1bec68795cae6478ee31f4f8c5348f0a568b397aaede8201311c380015b7033218b1ffd53dfd1ed75047e9db15b36d447ffc2a03629482b36cf5a8065ec8c53b9110db481b04b680ed3f3ab637c3c9be3fc3c3bb1e60fe590068e220b2adce4b1464b0db453f9238fe6d00fb0281807399828d2444c2f0917f648215610b906f1089b8f5da01584e4e721c8de5fd8d6e4a494a6450e32c97534a6cdfc0d48f0c8a73340287c6c48bccad5bf47077eb82d9028385a2d5560f9954b77809135c56ae8a049a199fe1d027851c3cf1de3ddadf748f1a2a62e7ce4a72f0cea9c2014a45581b067e961fb114642db6a6ff3502818100e297059b5a7347256c4ba42c7e8c6b309c039fb31a3c880a10922e15b7ed4f809331182ea0a086b83c4de6205e64ea8ac6029f5c0193a9ff136463c9c611c1369c90b0e958aca41d3e1a432cd76a64055e300b6b82cf084065b9351b45c10955791447bf578f2e60544d5d50187f903deaeea1e86e6c8c8c04151b5f5eaade52";
        let privk2 = "308204bc020100300d06092a864886f70d0101010500048204a6308204a20201000282010100b6ae502944279e82b34c4beb10f67dba10cd25fae819210acc8ed4e3d4a814e5aee3b00c688ebe843b47f766427e8c66dcb1136128b5a8ea44f16116cf8cc84cbb55f2f0c59b0ec1a1ce99b2bcd9f32743a1e24dcc43c6182c43d3584b0a357ed716d19d2a1b18d5028d11f301ae0615a5e1dceaa309985353d31f1421f9767b7fe811f2138af51d9034619585561d6384fc58998af1d75ce7b7b54f814973823b9f79e9889bd7cf3c6d7ef05b722bdf7b54f69d38123f4f7425b49681b996738374ee680a8ae445393ca20c1818fb89be340716ff9d6c15d0aff5e16d1f11f0bd8df540b8ed5c340c4e636ce642ee10aeb8e0059f7c16cfca024ed3970272e3020301000102820100451805faadabfc807bb742499cc756034f828038779bb58b23966c3fe5b952fa125d4cc34cb29cad5fcc96eea6fcbd36d486e7090b00366cb0f9c8da7b52c899790b8790f8746eaedef7c8db3921881d942f80ec22f38953b03e510be689ec74d67e6b76b1abc10723e95e5e16870f07161028e1d81b737124d5c7bdf221abe5637b043d7fbe753db1314056ef65c072ad36f256347eed583548dea9c6acaec60d4241c3b68c4eb5ff9395753765ffa5e1f6b059730934915d54f8c1526f91b8583acf42908bd55980bfde2d692515eb0ca311fe36e3a1e2dc0a52714a708531a48bfd52050c679cb62a784fbc35ccbf91cbdd624acc8ce6b95160dd3f17226102818100dca5d1aa43ad9871ad3775c4452e698d68191bad9734675c7bd4e8ee9c1ff1c05d1d3927fec729c3868784c59f816acbb355e5041af8465df0aa708253cea1fbfb4b11d0f4436f8c2eae5782bb0285911d65ac8588845e96f36344ea68359f504aff170960329d7a0e95a0c35541f53fbb4163456e8d8dc610ac149ea2131f0302818100d3f33b1dd06ee417cd69ef8f2d17ddcdd80155262bebf9e597fac7aed9d93577fbde150c93ad98cb0ceb5f854e1e224f376c0f4f1f168447af3ca328971b7fc4dc98c447a5cf8296ed72c3610e5d3ddd36647a25cc9d0f4451264ee81620075b02e4872f10a86563d5764e765b1a4bcd23bea231489ec43e8db219fa4ba0a6a102818001b1881d6d6d8ca8fab25d46075de6d37e040b5156c2c1345582f9d2b3020fc1f13503364a5f4ef3c039940c4c401b08bb34a2905880a5519d4241a0ce71dc8e698c56f3aa9c45e3e68bd2021fdb52191e07a4be55a0e674f42343e924a99cb26a10f1255246b12cb9a5ee58f17393254d13a0666d05cb1bc50efd0d86a2ecef028180244a100421cceac6dc87d7d986da00431f49d31f6f03bf4cbd41d5f0ad221092939049c05684b1958a87be5a1faeef26eb115869aea3f75022c3da17b80fa047bf917481e3f4eca214d3c27a1ab082481ee90334f79ca8a184d76f4933889659d1dbf8fd68f7bc2c64bf15de13e923b362fc5fdeda553cba8d1e426e6586832102818016df4e6d5dca014855917b6e9e7fcd82cf8e766c89ca1ade85205bd7ef465545367ec9139ff828a35076eadfd25a0f81cb31f44fa02bb2c6d21e2bf80fab52ca0a8f46b0864f0c2be5233a7d5c8911e0cb5ea51415be4da46235a08762d579b252b7a96470758257d320afa6d12eb8708108b49c4cf5140e94800a855c48ec44";
        (PeerInfo::new(privk1, port1), PeerInfo::new(privk2, port2))
    }

    #[test]
    fn test_request_response() {
        tracing_subscriber::fmt()
            .without_time()
            .with_target(false)
            .init();

        let (port1, port2) = (10002, 10003);
        let (peer1, peer2) = get_info(port1, port2);
        let mut allow_list = AllowHandler::new();
        let mut p2p1 = P2pHandler::new(peer1.address, &peer1.privk, allow_list.clone()).unwrap();
        let mut p2p2 = P2pHandler::new(peer2.address, &peer2.privk, allow_list.clone()).unwrap();
        allow_list.add(peer1.pubk_hash.clone(), None).unwrap();
        allow_list.add(peer2.pubk_hash.clone(), None).unwrap();
        let request_data = b"hello peer2".to_vec();
        let response_data = b"hello peer1".to_vec();

        // peer1 sends request to peer2
        p2p1.send(&peer2.pubk_hash, peer2.address, request_data.clone())
            .unwrap();

        // peer2 receives the request
        match p2p2.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, peer1.pubk_hash);
                assert_eq!(data, request_data);

                // peer2 sends a response back to peer1
                p2p2.send(&peer1.pubk_hash, peer1.address, response_data.clone())
                    .unwrap();
            }
            _ => panic!("Peer2 expected to receive a message"),
        }

        // peer1 receives the response
        match p2p1.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, peer2.pubk_hash);
                assert_eq!(data, response_data);
            }
            _ => panic!("Peer1 expected to receive a response"),
        }
    }
}
