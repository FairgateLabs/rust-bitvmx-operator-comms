# Bitvmx Operator Comms

## Overview

**BitVMX Operator Comms** provides a communication layer for direct connections between operators, built on top of the [Rust BitVMX Broker](https://github.com/FairgateLabs/rust-bitvmx-broker).  
It uses TLS certificates for authentication and public key based identifiers to establish secure channels between nodes.  

The main entry point is the `OperatorComms`, which offers a simple API for sending and receiving data, while the internal `Broker` manages certificates, storage, and routing.



## ⚠️ Disclaimer

This library is currently under development and may not be fully stable.
It is not production-ready, has not been audited, and future updates may introduce breaking changes without preserving backward compatibility.

## Features

- 🔐 **Secure connections** using TLS certificates (PEM private key required)  
- 🧾 **Identity verification** by public key hash  
- ✅ **Allowlist and routing table** support to control authorized peers and message flows  
- 📡 **Handler API** for simplified send/receive operations  
- 💾 **Persistent storage backend** for broker state  
- 🛑 **Graceful shutdown** via `stop()`  

## Methods  
The `OperatorComms` provides several methods to manage direct communication between operators:

- **new**: Creates a new handler bound to a socket address with a private key, allowlist, and routing table.  

- **check_receive**: Checks for incoming messages, returning the sender identifier and message if available.  

- **send**: Sends a message (bytes) to another operator, identified by its public key hash and socket address.  

- **stop**: Stops the handler and closes its broker connection.  

- **get_pubk_hash_from_privk**: Extracts the public key hash from a private key.  

- **get_pubk_hash**: Retrieves this handler’s own public key hash.  

- **get_address**: Returns the socket address the handler is bound to.  


## Usage

`OperatorComms` lets peers exchange messages over a TLS-based point-to-point channel.  
Peers are identified by their public key hash, and only allowed peers can communicate.

```rust
fn main() {
        // Inititialize peers
        let privk1 = "test_certs/priv1.key";
        let privk2 = "test_certs/priv2.key";
        let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 20000);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 20001);
        let cert1 = Cert::from_key_file(privk1).unwrap();
        let cert2 = Cert::from_key_file(privk2).unwrap();
        let pubk_hash1 = cert1.get_pubk_hash().unwrap();
        let pubk_hash2 = cert2.get_pubk_hash().unwrap();
        let identifier1 = Identifier::new_local(pubk_hash1, 0, 20000);
        let identifier2 = Identifier::new_local(pubk_hash2.clone(), 0, 20001);

        // Add peers to allow list and routing table
        let allow_list =
            AllowList::from_certs(vec![cert1, cert2], vec![addr1.ip(), addr2.ip()]).unwrap();
        let routing = RoutingTable::new();
        routing
            .lock()
            .unwrap()
            .add_route(identifier1.clone(), identifier2);

        // Initialize Operator Comms
        let mut comms1 =
            OperatorComms::new(addr1, &privk1, allow_list.clone(), routing.clone()).unwrap();
        let mut comms2 = OperatorComms::new(addr2, &privk2, allow_list.clone(), routing).unwrap();

        let msg = b"hello peer2".to_vec();

        // Peer1 sends message to peer2
        comms1.send(&pubk_hash2, addr2, msg.clone()).unwrap();

        // Peer2 receives the message
        match comms2.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, identifier1);
                assert_eq!(data, msg);
            }
            _ => panic!("Peer2 expected to receive a message"),
        }

        // Close the brokers
        comms1.stop().unwrap();
        comms2.stop().unwrap();
}
```
## Contributing
Contributions are welcome! Please open an issue or submit a pull request on GitHub.

## License
This project is licensed under the MIT License.

