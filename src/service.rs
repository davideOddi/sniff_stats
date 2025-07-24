use crate::model;
use model::Config;
use crate::util;
use crate::network_capture;
use std::path::PathBuf;
use std::thread;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::{Sender};
use std::collections::HashSet;  


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

pub fn monitor_network(config: Config) {
    let file_path = "/home/davide/rustRepo/sniff_stats/test.pcap";
    /*let packets: Vec<model::PacketData> = retrieve_data_packets(file_path);
    println!("Pacchetti catturati: {:?}", packets.len());
    let stats: model::NetworkStats = generate_network_stats(packets);
    println!("Statistiche di rete: {:?}", stats);
    let output_path = file_path.replace(".pcap", ".json");
    save_stats_to_file(&stats, &output_path);*/
      let (tx, rx) = std::sync::mpsc::channel::<PathBuf>();

    let _watcher = folder_monitoring_thread(PathBuf::from(config.watch_dir), tx)
        .expect("Errore nella creazione del watcher");

    let mut target_files: HashSet<PathBuf> = HashSet::new();

    for pcap_path in rx {
        if target_files.contains(&pcap_path) {
            continue;
        }
        target_files.insert(pcap_path.clone());
        println!("{:?} contenuto Set", target_files);
        println!("File ricevuto dal watcher: {:?}", pcap_path);
    }

}

fn work_process(input_path: String, output_path: String) -> Vec<model::PacketData> {
    let data_packets: Vec<model::PacketData> = retrieve_data_packets(&input_path);
    let stats: model::NetworkStats = generate_network_stats(&data_packets);
    save_stats_to_file(&stats, &output_path);
    return data_packets;
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

fn generate_network_stats(data_packets: &Vec<model::PacketData>) -> model::NetworkStats {
    return crate::stat_helper::generate_stats(data_packets);
}

fn folder_monitoring_thread(path: PathBuf, file_sender: Sender<PathBuf>) 
-> notify::Result<RecommendedWatcher>  {
     let (notify_tx, notify_rx) = std::sync::mpsc::channel();

    let mut watcher: RecommendedWatcher = notify::recommended_watcher(notify_tx)?;
    watcher.watch(&path, RecursiveMode::NonRecursive)?;

    println!("In ascolto sulla cartella: {}", path.display());

    thread::spawn(move || {
        for event_result in notify_rx {
            match event_result {
                Ok(event) => {
                    if matches!(event.kind, notify::EventKind::Create(_)) ||
                        matches!(event.kind, notify::EventKind::Modify(_))  {
                        for path in event.paths {
                            if path.extension().and_then(|e| e.to_str()) == Some("pcap") {
                                println!("📄 Nuovo file pcap: {:?}", path);
                                if let Err(e) = file_sender.send(path.clone()) {
                                    eprintln!("Errore invio path al canale: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Errore evento notify: {:?}", e),
            }
        }
    });

    Ok(watcher)
}