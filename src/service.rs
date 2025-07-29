use crate::job_dispatcher;
use crate::model;
use crate::model::NetworkStats;
use model::Config;
use crate::util;
use crate::network_capture;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::thread_helper;
use crate::model::PacketData;


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
    // questo il Producer per il watcher per inviare i percorsi dei file al thread principale
    let (watcher_file_tx, watcher_file_rx) = std::sync::mpsc::channel::<PathBuf>();

    // questa la coda per iinviare i job ai worker
    let (job_tx, job_rx) = std::sync::mpsc::channel::<(String, String)>(); // (input_path, output_path)

    // per creare tanti worker devo PER FORZA usare un Arc<Mutex<Receiver>> per il clone
    let shared_job_rx: Arc<Mutex<std::sync::mpsc::Receiver<(String, String)>>> = Arc::new(Mutex::new(job_rx));

    let raw_packets_list: Arc<Mutex<Vec<PacketData>>> = Arc::new(Mutex::new(Vec::new()));

    let watcher_result = thread_helper::folder_monitoring_thread(
        PathBuf::from(&config.watch_dir),
        watcher_file_tx);

    // mantiene il watcher in un thread separato e sempre attivo
    let _watcher = match watcher_result {
        Ok(w) => {
            println!("Watcher avviato e in ascolto su: {}", config.watch_dir);
            w // Il watcher è ora legato a `_watcher` e non viene droppato subito
        },
        Err(e) => {
            eprintln!("Errore nella creazione del watcher: {}", e);
            std::process::exit(1); 
        }
    };

    let raw_packets_arc: Arc<Mutex<Vec<PacketData>>> = Arc::clone(&raw_packets_list);
    let stats_path: String = format!("{}{}", config.output_dir, "/total_stats.json");
    let worker_handles: Vec<std::thread::JoinHandle<()>> = thread_helper::generate_workers(
        config.parallelism,
        shared_job_rx.clone(),
         move |input, output| {
            process_and_save_network_data(input, output, raw_packets_arc.clone(), &stats_path)
                .map_err(|e| format!("{}", e))
        },
        );
    // avvio il dispatcher dei job
    job_dispatcher::dispatch_jobs(
        watcher_file_rx,
        &job_tx,
        &config.output_dir,
    );

    println!("Chiusura dei job e attesa dei worker...");
    drop(job_tx);
    for (i, handle) in worker_handles.into_iter().enumerate() {
        if let Err(e) = handle.join() {
            eprintln!("Errore nel join del worker {}: {:?}", i, e);
        }
    }
    println!("Tutti i worker hanno terminato.");
}


fn work_process(input_path: String, output_path: String) -> Result<Vec<model::PacketData>, Box<dyn std::error::Error>> {
    let data_packets: Vec<model::PacketData> = retrieve_data_packets(&input_path)?;
    let stats: model::NetworkStats = generate_network_stats(&data_packets);
    save_stats_to_file(&stats, &output_path)?;
    Ok(data_packets)
}

fn save_stats_to_file(stats: &model::NetworkStats, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    util::write_json_file(file_path, stats)
        .map(|_| println!("Statistiche salvate in {}", file_path))
        .map_err(|e| {
            eprintln!("Errore nel salvataggio delle statistiche: {} per input file -> {}", e, file_path);
            e.into()
        })
}

fn retrieve_data_packets(file_path: &str) -> Result<Vec<model::PacketData>, Box<dyn std::error::Error>> {
    let packets = network_capture::pcap_reader(file_path)?;
    Ok(packets)
}

fn generate_network_stats(data_packets: &Vec<model::PacketData>) -> model::NetworkStats {
    return crate::stat_helper::generate_stats(data_packets);
}

fn write_stats(aggregated_stats: &NetworkStats, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    util::update_file(output_path, aggregated_stats)
        .map_err(|e| {
            eprintln!("Errore nell'aggiornamento del file delle statistiche: {}", e);
            e.into()
        })
}

fn process_and_save_network_data(
    input: String,
    output: String,
    raw_packets_arc: Arc<Mutex<Vec<PacketData>>>,
    stats_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match work_process(input, output) {
        Ok(local_packets) => {
            let stats = {
                let mut all = raw_packets_arc.lock().unwrap();
                all.extend(local_packets);
                generate_network_stats(&all)
            };

            // Salva le statistiche globali aggiornate
            if let Err(e) = write_stats(&stats, stats_path) {
                eprintln!("Errore nel salvataggio delle statistiche globali: {}", e);
            }

            Ok(())
        }
        Err(e) => {
            eprintln!("Errore nel process_save_packets: {}", e);
            Err(e)
        }
    }
}


