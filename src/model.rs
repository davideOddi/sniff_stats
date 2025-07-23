use std::collections::HashMap;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;

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

impl fmt::Display for InternetProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InternetProtocol::IPv4 => write!(f, "IPv4"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)] 
pub enum TransportProtocol {
    Tcp,
    Udp,
}

impl fmt::Display for TransportProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportProtocol::Tcp => write!(f, "Tcp"),
            TransportProtocol::Udp => write!(f, "Udp"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)] 
pub enum ApplicationProtocol {
    Dns, 
    Http, 
}

impl fmt::Display for ApplicationProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplicationProtocol::Dns => write!(f, "Dns"),
            ApplicationProtocol::Http => write!(f, "Http"),
        }
    }
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

#[derive(Debug, PartialEq, Eq, Hash, Deserialize)]
pub enum ProtocolKey {
    Internet(InternetProtocol),
    Transport(TransportProtocol),
    Application(ApplicationProtocol),
}

impl fmt::Display for ProtocolKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProtocolKey::Internet(p) => write!(f, "{}", p),
            ProtocolKey::Transport(p) => write!(f, "{}", p),
            ProtocolKey::Application(p) => write!(f, "{}", p),
        }
    }
}

impl Serialize for ProtocolKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct NetworkStats {
    pub total_packets: usize,
    pub total_bytes_packet: u64,
    pub by_protocol: HashMap<ProtocolKey, u32>,
    pub top_10_ips: Vec<String>,
    pub top_10_ports: Vec<u16>,
}
