use crate::model;
use model::Config;
use crate::util;
use crate::network_capture;


pub fn load_config() -> Config {
    const CONFIG_PATH: &str = "properties.json";
    let config: Config = 
    match util::read_json_file_as(CONFIG_PATH) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Errore nel caricamento della configurazione: {}", e);
            std::process::exit(1);
        }
    };
    return config;
}

pub fn monitor_network(){
    let file_path = "/home/davide/rustRepo/sniff_stats/test.pcap";
    let packets: Vec<model::PacketData> = retrieve_data_packets(file_path);
    println!("Pacchetti catturati: {:?}", packets.len());
    let stats: model::NetworkStats = generate_network_stats(packets);
    println!("Statistiche di rete: {:?}", stats);
    let output_path = file_path.replace(".pcap", ".json");
    save_stats_to_file(&stats, &output_path);
}

fn save_stats_to_file(stats: &model::NetworkStats, file_path: &str) {
    match util::write_json_file(file_path, stats) {
        Ok(_) => println!("Statistiche salvate in {}", file_path),
        Err(e) => 
            eprintln!("Errore nel salvataggio delle statistiche: {} per input file -> {}" 
            , e, file_path),
        
    }
}

fn retrieve_data_packets(file_path: &str)-> Vec<model::PacketData> {
    return network_capture::pcap_reader(file_path)
        .expect("Errore nella lettura del file PCAP");
}

fn generate_network_stats(data_packets: Vec<model::PacketData>) -> model::NetworkStats {
    return crate::stat_helper::generate_stats(data_packets);
}