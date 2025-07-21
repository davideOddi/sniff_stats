use crate::model::{PacketData, ProtocolType};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::Packet;
use pnet::packet::ip::IpNextHeaderProtocols;

/* implementazione blackboxed */
//toDo: implementare lettura dei pacchetti per bytes
pub fn packet_mapper(packet_data: &[u8]) -> Option<PacketData> {
    let ethernet: EthernetPacket<'_> = EthernetPacket::new(packet_data)?;

    if ethernet.get_ethertype() != EtherTypes::Ipv4 {
        return None;
    }

    let ipv4: Ipv4Packet<'_> = Ipv4Packet::new(ethernet.payload())?;
    let source_ip: String = ipv4.get_source().to_string();
    let length: usize = ipv4.packet().len();

    match ipv4.get_next_level_protocol() {
        IpNextHeaderProtocols::Tcp => {
            if let Some(tcp) = TcpPacket::new(ipv4.payload()) {
                let source_port = tcp.get_source();
                let proto = match source_port {
                    80 => ProtocolType::Http,
                    _ => ProtocolType::Tcp,
                };
                Some(PacketData {
                    protocol_type: proto,
                    source_ip,
                    source_port,
                    packet_length: length,
                })
            } else {
                None
            }
        }
        IpNextHeaderProtocols::Udp => {
            if let Some(udp) = UdpPacket::new(ipv4.payload()) {
                let source_port = udp.get_source();
                let proto = match source_port {
                    53 => ProtocolType::Dns,
                    _ => ProtocolType::Udp,
                };
                Some(PacketData {
                    protocol_type: proto,
                    source_ip,
                    source_port,
                    packet_length: length,
                })
            } else {
                None
            }
        }
        _ => Some(PacketData {
            protocol_type: ProtocolType::IPv4,
            source_ip,
            source_port: 0,
            packet_length: length,
        }),
    }
}