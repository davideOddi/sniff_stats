use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub watch_dir: String,
    pub output_dir: String,
    pub parallelism: i8,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)] 
pub enum InternetProtocol {
    IPv4,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)] 
pub enum TransportProtocol {
    Tcp,
    Udp,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)] 
pub enum ApplicationProtocol {
    Dns, 
    Http, 
}

#[derive(Debug)] 
pub struct PacketData {
    pub internet_layer: InternetProtocol, 
    pub transport_layer: Option<TransportProtocol>,
    pub application_layer: Option<ApplicationProtocol>,
    pub source_ip: String,
    pub destination_ip: String,
    pub source_port: u16,
    pub destination_port: u16,
    pub packet_length: usize,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolKey {
    Internet(InternetProtocol),
    Transport(TransportProtocol),
    Application(ApplicationProtocol),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkStats {
    pub total_packets: usize,
    pub total_bytes_packet: u64,
    pub by_protocol: HashMap<ProtocolKey, u32>,
    pub top_10_ips: Vec<String>,
    pub top_10_ports: Vec<u16>,
}
