use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum P2pHandlerError {
    #[error("Internal error: {0}")]
    Error(String),

    #[error("Broker error: {0}")]
    BrokerError(String),
}

#[derive(Debug, PartialEq)]
pub enum ReceiveHandlerChannel {
    Msg(String, Vec<u8>), //Id, Msg
    Error(P2pHandlerError),
}
