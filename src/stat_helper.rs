use crate::model::PacketData;
use crate::model::NetworkStats;
use crate::model::ProtocolKey;
use std::collections::{BTreeMap, BTreeSet, HashMap};


fn invert_key_value<T: Ord + Clone>(map: BTreeMap<T, u32>) -> BTreeMap<u32, BTreeSet<T>> {
    let mut frequency_map: BTreeMap<u32, BTreeSet<T>> = BTreeMap::new();
    for (key, freq) in map {
        frequency_map.entry(freq).or_default().insert(key);
    }
    return frequency_map;
}

fn top_n_by_frequency<T: Ord + Clone>(map: BTreeMap<T, u32>, n: usize) -> Vec<T> {
    let inverted = invert_key_value(map);
    let mut result = Vec::new();

    for (_, keys) in inverted.iter().rev() { 
        for key in keys {
            result.push(key.clone());
            if result.len() >= n {
                return result;
            }
        }
    }

    return result;
}

pub fn generate_stats(data_packets: &Vec<PacketData>) -> NetworkStats {
    let mut stats = NetworkStats {
        total_packets: 0,
        total_bytes_packet: 0,
        by_protocol: HashMap::new(),
        top_10_ips: Vec::new(),      
        top_10_ports: Vec::new(),
    };

    let mut ip_freq: BTreeMap<String, u32> = BTreeMap::new();
    let mut port_freq: BTreeMap<u16, u32> = BTreeMap::new();

    stats.total_packets = data_packets.len();

    for packet in data_packets.iter() {
        stats.total_bytes_packet += packet.packet_length as u64;

        *stats.by_protocol.entry(ProtocolKey::Internet(packet.internet_layer.clone()))
            .or_insert(0) += 1;

        if let Some(transport_layer) = packet.transport_layer.clone() {
            *stats.by_protocol.entry(ProtocolKey::Transport(transport_layer))
                .or_insert(0) += 1;
        }

        if let Some(application_layer) = packet.application_layer.clone() {
            *stats.by_protocol.entry(ProtocolKey::Application(application_layer))
                .or_insert(0) += 1;
        }

        *ip_freq.entry(packet.source_ip.clone()).or_insert(0) += 1;
        *ip_freq.entry(packet.destination_ip.clone()).or_insert(0) += 1;

        *port_freq.entry(packet.source_port).or_insert(0) += 1;
        *port_freq.entry(packet.destination_port).or_insert(0) += 1;
    }

    stats.top_10_ips = top_n_by_frequency(ip_freq, 10);
    stats.top_10_ports = top_n_by_frequency(port_freq, 10);

    return stats;
}
