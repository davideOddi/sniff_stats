mod util;
mod service;
mod model;
mod network_capture;
mod pcap_helper;

fn main() {
    let config: model::Config = service::load_config();
    println!("Configurazione caricata: {:?}", config.output_dir);
    let packets: Vec<model::PacketData> = service::monitor_network();
    println!("Pacchetti catturati: {:?}", packets.len());
}
