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

pub fn monitor_network()-> Vec<model::PacketData> {
    return network_capture::pcap_reader("/home/davide/rustRepo/sniff_stats/test.pcap")
        .expect("Errore nella lettura del file PCAP");
}