use crate::broker::Broker;
use crate::helper::*;
use bitvmx_broker::rpc::tls_helper::get_pubk_hash_from_privk;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tracing::{error, info};

pub use bitvmx_broker::identification::{allow_list::AllowList, routing::RoutingTable};

pub struct P2pHandler {
    broker: Broker,
}

pub type PubKeyHash = String;

impl P2pHandler {
    pub fn new(
        address: SocketAddr,
        privk: &str, // PEM format
        allow_list: Arc<Mutex<AllowList>>,
        routing: Arc<Mutex<RoutingTable>>,
    ) -> Result<Self, P2pHandlerError> {
        let broker = Broker::new(address, privk, allow_list, routing)
            .map_err(|e| P2pHandlerError::Error(format!("Failed to create broker: {e}")))?;
        Ok(P2pHandler { broker })
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
            Some((id, data)) => {
                let data = serde_json::from_str::<Vec<u8>>(&data.to_string())
                    .map_err(|e| P2pHandlerError::Error(e.to_string()))?;
                info!("Receive data from id: {}: {:?}", id, data);
                Ok(Some(ReceiveHandlerChannel::Msg(id, data)))
            }
            None => Ok(None),
        }
    }

    pub fn send(
        &self,
        pubk_hash: &PubKeyHash,
        address: SocketAddr,
        data: Vec<u8>,
    ) -> Result<(), P2pHandlerError> {
        let data =
            serde_json::to_string(&data).map_err(|e| P2pHandlerError::Error(e.to_string()))?;

        self.broker
            .put(
                address.port(),
                Some(address.ip()),
                pubk_hash.to_string(),
                data,
            )
            .map_err(|e| P2pHandlerError::BrokerError(e.to_string()))?;

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), P2pHandlerError> {
        self.broker.close();
        Ok(())
    }

    // Private Key in PEM format
    pub fn get_pubk_hash_from_privk(privk: &str) -> Result<PubKeyHash, P2pHandlerError> {
        let pk_hash = get_pubk_hash_from_privk(privk)
            .map_err(|e| P2pHandlerError::BrokerError(e.to_string()))?;
        Ok(pk_hash)
    }

    pub fn get_pubk_hash(&self) -> Result<PubKeyHash, P2pHandlerError> {
        self.broker
            .get_pubk_hash()
            .map_err(|e| P2pHandlerError::BrokerError(e.to_string()))
    }

    pub fn get_address(&self) -> SocketAddr {
        self.broker.get_address()
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use crate::broker::COMMS_ID;

    use super::*;
    use bitvmx_broker::{identification::identifier::Identifier, rpc::tls_helper::Cert};
    use std::net::{IpAddr, Ipv4Addr};
    use tracing_subscriber::{
        fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
    };

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct PeerInfo {
        privk: String,
        pubk_hash: String,
        address: SocketAddr,
    }
    impl PeerInfo {
        fn new(privk: &str, port: u16) -> Self {
            let cert = Cert::new_with_privk(privk).unwrap();
            let pubk_hash = cert.get_pubk_hash().unwrap();
            let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
            let address = SocketAddr::new(ip, port);
            Self {
                privk: privk.to_string(),
                pubk_hash,
                address,
            }
        }

        fn get_identifier(&self) -> Identifier {
            Identifier {
                pubkey_hash: self.pubk_hash.clone(),
                id: Some(COMMS_ID),
                address: self.address,
            }
        }
    }
    fn get_info(port1: u16, port2: u16) -> (PeerInfo, PeerInfo) {
        let privk1 = "b'-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDhzkbFynswfys/\nVNbM4hzYNKCdAuxYI/jysOPkRHGhlJe+71EE9F2CpAZnjevBsUWxi3+LatfMZjwi\nUz/l3iC6ow8Dsar0BO6RmWQR8Uf/1sx+WNjBk2woISPb60oXbXYj8AVUqYUUSo/Q\nRF5kuGT7dsMvUAx8Irn93w4A5VXx+FLn3r38Tymv7qOMT5cO1xrNStsluBD1RdPj\nz+B6b+7woAKqkrNFR+ZH0HUUKldA+A+pGElQLODyLB7OwxHgKtEsFdyiiDuKW2mP\nsk2dsab9HCNdo9cViA9UbeykDXq7h0/7gYg9XBH8LqqXYpSk/LE6T8k1RVa9EBxV\nRpYqlvFPAgMBAAECggEAV64pfRQq0aIPwP/IiLYkTS/iThWcgH03ZcWaOED7fqqc\nYd+7rhjVVq0qb3uEWCnlzhNE63YJZa0tHIcHANNIEjDO27hZkXd4y8CsQutV8doO\nfeEyCbic/tgffH3Yv1AZ18qTx1QsAL0TKuPhY2rWi26KTAzhTDKP1iyO23ox7Uqs\nwWChuHWyw7SmECRmjKOjTLs1Axea3fos6ERgEv/KZiTi+a9he5JuHOXO6aKTvHI7\nlTAMdloy1CnK6G3Ql7LfBeX20hIwDSZNgp5naB6NjJiDTbxxlGj7apW6hquzJpRP\n1Tn2YLvVKl5bdAOHh44wHBhZR9COjxUT+uASYRb5wQKBgQD7FTe3VPrsi6ejo7db\n9SwTUjsTQKoxrfoNc0xPzGGwKyyArGM++NQI1CZuQQDXVoYl+JC1JOcTLjjW/TYu\nwVGAr63bjtYjU0e8NZzum3nIZ7rpyHJpnbCLBc678KNCvblD4u/Vl1bx/9vRiCTx\n9S0r/LJ54Jr3Ohx9feYERc4K/QKBgQDmOlWNHwFlC2pkYI/0biXWybQZWvz+C5x3\nJO6tf0ykRk2sBEcp07JMhJsE+r4B+lHNSWalkX409Fn6x2ch/6tLP0X+viM5nr+2\nRpGHLpUBeq4+RKMmUS/NgY2DoRV1DRnfk4Vt0BZy5Voc4OVQz0zohwFzYhY60ThR\nV3UJ9HbdOwKBgQCcBS8+CNxzqMRe9xi1V8AvsWVsLT6U6Fr9iKve2k3JvspEmtqB\nAvYfFlVbJaF0Lhvl9HNXXLsKPCqtzWKh4xbWNFSAnl2KTfHBjj8aNhqS4YJQS3Jt\nFsPhX5Z7SqjojCRXfukxfH1Wm3ro1QTAJW4Qa1IsUdl5zu5tPJJ2DTpfsQKBgCii\nXR0mPsnFxQZoYKAEnNsXCJl9DLAN/pSsyQ+IK0/HNMhKjQDd41dMBExRsR2KP8va\ny6onTr4r7oGrlhFTHbmPNlxq1K7DzRRvyhmw6A21yHEnDiCiLay40/BKiw34vPtP\n/znNg1jOECSOsQqdO/bCdUgXJNNGwAjjRb33Ds+nAoGAW76wLk1lwD2tZ8KgMRUU\ni0BkY7eDXPskxCP6BjFq10J/1dC/dsLO9mZfwl2BJ2D+gGmcIzdSb5p1LkuniGuv\nV+/lSa8bdUKwtd5l+CZ0OMqmHryQZICqGeG5uREYv5eqs4mDiuM8QkZdOZUKWzPc\nwWJXrp5cQtvgjS/HyjHB69o=\n-----END PRIVATE KEY-----\n'";
        let privk2 = "b'-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCeJYILLK2EpGP9\nCrlEeHL1hYODftAUxJTacRezNNuyAqqP04H0IFffXhdz/f54HnYnaN1VrMGNQlR5\nBashFjZa7fVEFp3osVgNEPNu63MA1Gr7o4BakopRbMx7jUyhmlJXNP3VX5tZEha+\nV7GOZEeh2Ej3pehnE/E6SD16Ez9aaGydFgrMALHjT2NfucK0XCcDvMbq53PsBaLm\nnH5TLnvtZvYmdyDoUe+RvlwaRAHv4AWDOElhQrj970giHWY6i9QgqrlTIYN5cQrD\nM6kNj1SaBtCNpG/wIK3NMLW7PAYeEKTopwdsFuVL+1e0IAsTIVpDC1mb3r2GlPji\n0GaMLBAHAgMBAAECggEAFPHDvMYgfuIkqeULL1HCa9jQV5Bqb22vhxSWPnIgLH2k\n6CJrYhEMgjUcZwy68F6caFC/i3KzOYmQ1WxWQy4Fadp88pUKOcCO+EAH9WcyVmhL\neOMpAxXIQstlc3F9tiNRh2IpweIFGXFHWNMVXVXTlNAnrcCnvEsMVhsuJSY6bDcV\n5ejQKE8kM8F30FzD2mii36XamsreMpQBAIlm0i1HH/8PpynUQ12bb2M0T/FR9C5V\nAbfeLUOgrzWgBs9hxmlBzILusJFjv7OvwIkF97GgoAyLKqFmxzncwQUTqh9iH2Js\nemN6Qg+vPIg2Et8Ku9XEX+CSXvDwFckB2Z14jqQw8QKBgQDPHDzAFDSTl+aPH+vd\n01wxtaFyP7KP2OaRabW1qzTPww87agbN3wPJqBBf9lEjVeGNjLrp2NyHX6Wfnt5V\nlpeWts13/M43rju2JJwOrfZnwJsJgQ9ZEQw30e1LWeiGpr0kcWlv2059tEiKgBwY\nNlw6evsCyFjrIuSqgg3riO9xMQKBgQDDel5TfTJ3BJZlgFYnU1YxUZDQ1mcMDnSK\ntdRLdpVWTEkjzf0a6rGJYla0NoqQdH9qDfimVMY6+RQLZVdhhXDVnQuwV0CK9ERY\nQWy/PEoPvIagTXgKJ8fKLYcG420fJJtPmTSEoPZg1PXtuABNj/68bI7ONL5CY6gO\n8iFJU0sGtwKBgA6mlLWRuFZofGrLe0fp16+8hXsrflomocjPjYcYYVgBGGa/jVOq\n3v244c+oAP1a6eW1etNn/9GjtnegKWIskPScYdSHEZ9mt9qepFt1euTD/zOg6ZEH\nX7HjK8IUzhoYWXDmhOrgvKCvzCHgBhzAW63XXUJJIeEgSsS1Bn8O5MFBAoGAMuiv\noDa+6dg8AvtFdMBzdiyz9m+gLrelCmsIew7LHcqIUdbX0CbHTexagFykAbMVa91v\noIH7jmhIHB+sfi1ukXNxE9/lY0rycbm4RKXC9A45UY5bcOmjUrhArj6UsMOr3zMb\nRl9VSyqrUdnV2l1iDliHaJS76DZkEmBk4t/abkkCgYEAxkk3skKgRJPt2bFLzdHV\n3Au24P/Cyqf1LIfXpuJcMBfAhw55g6DOLR4O0BH+s7cZk8hrGVeI9WyhC5EgzZrF\nBjTlZFqFtsz5psj1oNqgr/JnO2fL3csxbDR81q9uSSzdlN7BlzBpdQahi53K9MHi\nZDNGUy5a/PopNnWSzfHYUas=\n-----END PRIVATE KEY-----\n'";
        (PeerInfo::new(privk1, port1), PeerInfo::new(privk2, port2))
    }
    fn add_allow_list(allow_list: Arc<Mutex<AllowList>>, peers: Vec<PeerInfo>) {
        for peer in peers {
            allow_list
                .lock()
                .unwrap()
                .add(peer.pubk_hash, peer.address.ip());
        }
    }
    #[test]
    fn test_boker() {
        init_tracing().unwrap();
        let data = "hello".to_string();
        let (port1, port2) = (10000, 10001);
        let (peer1, peer2) = get_info(port1, port2);
        let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port1);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port2);
        let allow_list = AllowList::new();
        let routing = RoutingTable::new();
        routing.lock().unwrap().allow_all();
        let mut broker1 =
            Broker::new(addr1, &peer1.privk, allow_list.clone(), routing.clone()).unwrap();
        let mut broker2 =
            Broker::new(addr2, &peer2.privk, allow_list.clone(), routing.clone()).unwrap();
        add_allow_list(allow_list.clone(), vec![peer1.clone(), peer2.clone()]);

        broker1
            .put(port2, None, peer2.pubk_hash, data.clone())
            .unwrap();
        let received_data = broker2.get().unwrap();
        assert_eq!(received_data, Some((peer1.get_identifier(), data)));
        broker1.close();
        broker2.close();
    }

    #[test]
    fn test_request_response() {
        let (port1, port2) = (10002, 10003);
        let (peer1, peer2) = get_info(port1, port2);
        let allow_list = AllowList::new();
        let routing = RoutingTable::new();
        routing.lock().unwrap().allow_all();
        let mut p2p1 = P2pHandler::new(
            peer1.address,
            &peer1.privk,
            allow_list.clone(),
            routing.clone(),
        )
        .unwrap();
        let mut p2p2 =
            P2pHandler::new(peer2.address, &peer2.privk, allow_list.clone(), routing).unwrap();
        add_allow_list(allow_list.clone(), vec![peer1.clone(), peer2.clone()]);
        let request_data = b"hello peer2".to_vec();
        let response_data = b"hello peer1".to_vec();

        // peer1 sends request to peer2
        p2p1.send(&peer2.pubk_hash, peer2.address, request_data.clone())
            .unwrap();

        // peer2 receives the request
        match p2p2.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, peer1.get_identifier());
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
                assert_eq!(from_id, peer2.get_identifier());
                assert_eq!(data, response_data);
            }
            _ => panic!("Peer1 expected to receive a response"),
        }

        // Close the brokers
        p2p1.stop().unwrap();
        p2p2.stop().unwrap();
    }

    #[test]
    fn test_concurrent_requests() {
        let (port1, port2) = (10004, 10005);
        let (peer1, peer2) = get_info(port1, port2);
        let allow_list = AllowList::new();
        let routing = RoutingTable::new();
        routing.lock().unwrap().allow_all();
        let mut p2p1 = P2pHandler::new(
            peer1.address,
            &peer1.privk,
            allow_list.clone(),
            routing.clone(),
        )
        .unwrap();
        let mut p2p2 =
            P2pHandler::new(peer2.address, &peer2.privk, allow_list.clone(), routing).unwrap();
        add_allow_list(allow_list.clone(), vec![peer1.clone(), peer2.clone()]);

        let request_data_1 = b"hello peer2".to_vec();
        let response_data_1 = b"hello peer1".to_vec();

        let request_data_2 = b"ping".to_vec();
        let response_data_2 = b"pong".to_vec();

        // peer1 sends request to peer2
        p2p1.send(&peer2.pubk_hash, peer2.address, request_data_1.clone())
            .unwrap();

        // peer2 sends request to peer1
        p2p2.send(&peer1.pubk_hash, peer1.address, request_data_2.clone())
            .unwrap();

        match p2p2.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, peer1.get_identifier());
                assert_eq!(data, request_data_1);

                // peer2 sends a response back to peer1
                p2p2.send(&peer1.pubk_hash, peer1.address, response_data_1.clone())
                    .unwrap();
            }
            _ => panic!("Peer2 expected to receive a message"),
        }

        //peer1 receives the request
        match p2p1.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, peer2.get_identifier());
                assert_eq!(data, request_data_2);

                // peer1 sends a response back to peer2
                p2p1.send(&peer2.pubk_hash, peer2.address, response_data_2.clone())
                    .unwrap();
            }
            _ => panic!("Peer1 expected to receive a message"),
        }

        // peer1 receives the response
        match p2p1.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, peer2.get_identifier());
                assert_eq!(data, response_data_1);
            }
            _ => panic!("Peer1 expected to receive a response"),
        }

        // peer2 receives the response
        match p2p2.check_receive() {
            Some(ReceiveHandlerChannel::Msg(from_id, data)) => {
                assert_eq!(from_id, peer1.get_identifier());
                assert_eq!(data, response_data_2);
            }
            _ => panic!("Peer2 expected to receive a response"),
        }

        // Close the brokers
        p2p1.stop().unwrap();
        p2p2.stop().unwrap();
    }

    #[test]
    fn test_channel() {
        let (port1, port2) = (10006, 10007);
        let (peer1, peer2) = get_info(port1, port2);
        let allow_list = AllowList::new();
        let routing = RoutingTable::new();
        routing.lock().unwrap().allow_all();
        let mut p2p1 = P2pHandler::new(
            peer1.address,
            &peer1.privk,
            allow_list.clone(),
            routing.clone(),
        )
        .unwrap();
        let mut p2p2 =
            P2pHandler::new(peer2.address, &peer2.privk, allow_list.clone(), routing).unwrap();
        add_allow_list(allow_list.clone(), vec![peer1.clone(), peer2.clone()]);

        let data = b"hello peer2".to_vec();
        p2p1.send(&peer2.pubk_hash, peer2.address, data.clone())
            .unwrap();
        assert_eq!(
            p2p2.check_receive(),
            Some(ReceiveHandlerChannel::Msg(
                peer1.get_identifier(),
                data.clone()
            ))
        );
        assert_eq!(p2p2.check_receive(), None);
        assert_eq!(p2p1.check_receive(), None);

        // Close the brokers
        p2p1.stop().unwrap();
        p2p2.stop().unwrap();
    }

    #[test]
    fn test_allow_list() {
        let (port1, port2) = (10008, 10009);
        let (peer1, peer2) = get_info(port1, port2);
        let allow_list = AllowList::new();
        let routing = RoutingTable::new();
        routing.lock().unwrap().allow_all();
        let mut p2p1 = P2pHandler::new(
            peer1.address,
            &peer1.privk,
            allow_list.clone(),
            routing.clone(),
        )
        .unwrap();
        let mut p2p2 =
            P2pHandler::new(peer2.address, &peer2.privk, allow_list.clone(), routing).unwrap();
        add_allow_list(allow_list.clone(), vec![peer1.clone()]);

        let result = p2p1.send(&peer2.pubk_hash, peer2.address, b"hello peer2".to_vec());
        assert!(matches!(result, Err(P2pHandlerError::BrokerError(_))));
        let result = p2p2.send(&peer1.pubk_hash, peer1.address, b"hello peer1".to_vec());
        assert!(matches!(result, Err(P2pHandlerError::BrokerError(_))));
        assert!(p2p1.check_receive().is_none());
        assert!(p2p2.check_receive().is_none());

        add_allow_list(allow_list.clone(), vec![peer2.clone()]);

        p2p1.send(&peer2.pubk_hash, peer2.address, b"hello peer2".to_vec())
            .unwrap();
        assert_eq!(
            p2p2.check_receive(),
            Some(ReceiveHandlerChannel::Msg(
                peer1.get_identifier().clone(),
                b"hello peer2".to_vec()
            ))
        );

        // Close the brokers
        p2p1.stop().unwrap();
        p2p2.stop().unwrap();
    }

    #[test]
    fn test_reconnecting() {
        let (port1, port2) = (10010, 10011);
        let (peer1, peer2) = get_info(port1, port2);
        let allow_list = AllowList::new();
        let routing = RoutingTable::new();
        routing.lock().unwrap().allow_all();
        let mut p2p1 = P2pHandler::new(
            peer1.address,
            &peer1.privk,
            allow_list.clone(),
            routing.clone(),
        )
        .unwrap();
        let mut p2p2 = P2pHandler::new(
            peer2.address,
            &peer2.privk,
            allow_list.clone(),
            routing.clone(),
        )
        .unwrap();
        add_allow_list(allow_list.clone(), vec![peer1.clone(), peer2.clone()]);

        let data = b"hello peer2".to_vec();

        p2p1.send(&peer2.pubk_hash, peer2.address, data.clone())
            .unwrap();

        assert_eq!(
            p2p2.check_receive(),
            Some(ReceiveHandlerChannel::Msg(
                peer1.get_identifier().clone(),
                data.clone()
            ))
        );

        // Simulate a reconnect by closing and reopening the broker
        p2p1.stop().unwrap();
        let mut p2p1 =
            P2pHandler::new(peer1.address, &peer1.privk, allow_list.clone(), routing).unwrap();

        // Check if we can still send messages after reconnecting
        p2p1.send(&peer2.pubk_hash, peer2.address, data.clone())
            .unwrap();

        assert_eq!(
            p2p2.check_receive(),
            Some(ReceiveHandlerChannel::Msg(peer1.get_identifier(), data))
        );

        // Close the brokers
        p2p1.stop().unwrap();
        p2p2.stop().unwrap();
    }

    pub fn init_tracing() -> anyhow::Result<()> {
        let filter = EnvFilter::builder()
            .parse("info,tarpc=off") // Include everything at "info" except `libp2p`
            .expect("Invalid filter");

        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::NEW | FmtSpan::CLOSE))
            .try_init()?;
        Ok(())
    }
}
