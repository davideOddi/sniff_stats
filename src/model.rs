use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub watch_dir: String,
    pub output_dir: String,
    pub parallelism: i8,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)] 
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

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkStats {
    pub total_packets: usize,
    pub total_bytes_packet: u64,
    pub by_protocol: HashMap<ProtocolType, u32>,
    pub top_10_ips: Vec<String>,
    pub top_10_ports: Vec<u16>,
}
