use crate::model::{PacketData, InternetProtocol, TransportProtocol, ApplicationProtocol};

const ETHERTYPE_IPV4: u16 = 0x0800;
const IP_VERSION_IPV4: u8 = 4;
const TRANSPORT_TCP: u8 = 0x06;
const TRANSPORT_UDP: u8 = 0x11;
const PORT_HTTP: u16 = 80;
const PORT_HTTPS: u16 = 443;
const PORT_DNS: u16 = 53;

pub fn packet_mapper(packet_data: &[u8]) -> Option<PacketData> {
    let packet_len: usize = packet_data.len();

    if packet_len < 34 {
        return None; 
    }

    let ethernet: u16 = u16::from_be_bytes([packet_data[12], packet_data[13]]);
    if ethernet != ETHERTYPE_IPV4 {
        return None; 
    }

    let ip_offset: usize = 14; 
    let version = packet_data[ip_offset] >> 4;
    if version != IP_VERSION_IPV4 {
        return None;
    }

    let ihl: u8 = (packet_data[ip_offset] & 0x0F) * 4;
    let transport_offset = ip_offset + ihl as usize;

    if packet_len < transport_offset + 4 {
        return None; 
    }

    let protocol: u8 = packet_data[ip_offset + 9];
    let source_ip: String = format!(
        "{}.{}.{}.{}",
        packet_data[ip_offset + 12],
        packet_data[ip_offset + 13],
        packet_data[ip_offset + 14],
        packet_data[ip_offset + 15]
    );
    let destination_ip = format!(
        "{}.{}.{}.{}",
        packet_data[ip_offset + 16],
        packet_data[ip_offset + 17],
        packet_data[ip_offset + 18],
        packet_data[ip_offset + 19]
    );

    let source_port = u16::from_be_bytes([
        packet_data[transport_offset],
        packet_data[transport_offset + 1],
    ]);
    let destination_port = u16::from_be_bytes([
        packet_data[transport_offset + 2],
        packet_data[transport_offset + 3],
    ]);

    let (transport_layer, application_layer) = match protocol {
        TRANSPORT_TCP => {
            let application_protocol = match destination_port {
                PORT_HTTP => Some(ApplicationProtocol::Http),
                PORT_HTTPS => Some(ApplicationProtocol::Http),
                PORT_DNS => Some(ApplicationProtocol::Dns),
                _ => None,
            };
            (Some(TransportProtocol::Tcp), application_protocol)
        }
        TRANSPORT_UDP => {
            let application_protocol = match destination_port {
                PORT_DNS => Some(ApplicationProtocol::Dns),
                _ => None,
            };
            (Some(TransportProtocol::Udp), application_protocol)
        }
        _ => return None,
    };

    Some(PacketData {
        internet_layer: InternetProtocol::IPv4,
        transport_layer,
        application_layer,
        source_ip,
        destination_ip,
        source_port,
        destination_port,
        packet_length: packet_len,
    })
}


