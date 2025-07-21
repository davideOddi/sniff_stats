use crate::pcap_helper::packet_mapper;
use crate::model::PacketData;
use pcap::{Capture};
use serde::ser::StdError;

pub fn pcap_reader(file_path: &str) -> Result<Vec<PacketData>, Box<dyn StdError>> {
    print!("Lettura file: {} ", file_path);

    let mut capture = match Capture::from_file(file_path) {
        Ok(cap) => cap,
        Err(e) => return Err(format!("{} nell'apertura del file PCAP {}",
          e, file_path,).into()),
    };

    let mut data_packets: Vec<PacketData> = Vec::new();

    while let Ok(packet) = capture.next() {

        if let Some(packet_data) = packet_mapper(packet.data) {
            data_packets.push(packet_data);
        }

    }
    return Ok(data_packets);

    
}