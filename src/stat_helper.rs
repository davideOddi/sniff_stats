use crate::model::PacketData;
use crate::model::NetworkStats;
use std::collections::HashMap;
use crate::model::ProtocolKey;

pub fn generate_stats(data_packets: &Vec<PacketData>) -> NetworkStats{
    let mut stats = NetworkStats {
        total_packets: 0,
        total_bytes_packet: 0,
        by_protocol: HashMap::new(),
        top_10_ips: Vec::new(),      
        top_10_ports: Vec::new(),
    };
    let mut ip_frequency_map: HashMap<String, u32> = HashMap::new();
    let mut port_frequency_map: HashMap<u16, u32> = HashMap::new();

    stats.total_packets = data_packets.len();

    for packet in data_packets.iter() {
        stats.total_bytes_packet += packet.packet_length as u64;
        // Incremento le statistiche per ogni layer del protocollo 
        *stats.by_protocol.entry(ProtocolKey::Internet
            (packet.internet_layer.clone())).or_insert(0) += 1;

        if let Some(transport_layer) = packet.transport_layer.clone() {
            *stats.by_protocol.entry(ProtocolKey::Transport(transport_layer))
                .or_insert(0) += 1;
        }
        if let Some(application_layer) = packet.application_layer.clone() {
            *stats.by_protocol.entry(ProtocolKey::Application(application_layer))
                .or_insert(0) += 1;
        }   

        *ip_frequency_map.entry(packet.source_ip.clone()).or_insert(0) += 1;
        *ip_frequency_map.entry(packet.destination_ip.clone()).or_insert(0) += 1;

        *port_frequency_map.entry(packet.source_port).or_insert(0) += 1;
        *port_frequency_map.entry(packet.destination_port).or_insert(0) += 1;

    }
    
    stats.top_10_ips = sort_values_by_frequency(ip_frequency_map);  
    stats.top_10_ports = sort_values_by_frequency(port_frequency_map); 

    return stats
}

fn sort_values_by_frequency<T: Eq + std::hash::Hash>(map: HashMap<T, u32>,) 
-> Vec<T> {
    let mut vec: Vec<_> = map.into_iter().collect();
    vec.sort_by(|a, b| b.1.cmp(&a.1));
    return vec.into_iter().take(10).map(|(k, _)| k).collect();
}
