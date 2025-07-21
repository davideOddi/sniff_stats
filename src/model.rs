use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub watch_dir: String,
    pub output_dir: String,
    pub parallelism: i8,
}


#[derive(Debug, PartialEq)] 
pub enum ProtocolType {
    IPv4,
    Tcp,
    Udp,
    Dns, 
    Http, 
}

#[derive(Debug)] 
pub struct PacketData {
    pub protocol_type: ProtocolType, 
    pub source_ip: String,
    pub source_port: u16,
    pub packet_length: usize,
}

