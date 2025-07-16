use bitvmx_broker::allow_list::AllowList;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub struct AllowHandler {
    allow_list: Arc<Mutex<AllowList>>,
}
impl AllowHandler {
    pub fn new() -> Self {
        let allow_list = AllowList::new();
        Self { allow_list }
    }
    pub fn from_yaml(path: &str) -> Result<Self, anyhow::Error> {
        let allow_list = AllowList::from_file(path)?;
        Ok(Self { allow_list })
    }
    pub fn get_allow_list(&self) -> Arc<Mutex<AllowList>> {
        Arc::clone(&self.allow_list)
    }
    pub fn add(
        &mut self,
        pubk_hash: String,
        addr: Option<SocketAddr>,
    ) -> Result<(), anyhow::Error> {
        let ip = match addr {
            Some(addr) => addr.ip(),
            None => IpAddr::V4(Ipv4Addr::LOCALHOST),
        };
        self.allow_list
            .lock()
            .map_err(|e| anyhow::Error::msg(e.to_string()))?
            .add(pubk_hash, ip);
        Ok(())
    }
    pub fn remove(&mut self, pubk_hash: &str) {
        self.allow_list.lock().unwrap().remove(pubk_hash);
    }
}
