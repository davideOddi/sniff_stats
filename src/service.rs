use crate::job_dispatcher;
use crate::model;
use crate::model::NetworkStats;
use model::Config;
use crate::util;
use crate::network_capture;
use std::path::PathBuf;
use crate::thread_helper;
use crate::model::PacketData;
use std::sync::{mpsc::{channel, Sender}, Arc, Mutex};

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
    // 1. Watcher → manda file al dispatcher
    let (watcher_file_tx, watcher_file_rx) = channel::<PathBuf>();

    // 2. Dispatcher → manda job (input, output) ai worker
    let (job_tx, job_rx) = channel::<(String, String)>();
    let shared_job_rx = Arc::new(Mutex::new(job_rx));

    // 3. Worker → manda batch di pacchetti all’aggregatore
    let (packet_tx, packet_rx) = channel::<Vec<PacketData>>();
    let packet_tx = Arc::new(packet_tx);

    // 4. Aggregatore → raccoglie i pacchetti in uno stato globale
    let stats_path = format!("{}/total_stats.json", config.output_dir);
    let initial_state: Vec<PacketData> = Vec::new();

    let update_state = |state: &mut Vec<PacketData>, batch: Vec<PacketData>| {
        state.extend(batch);
    };

    let save_stats = move |state: &Vec<PacketData>| -> Result<(), Box<dyn std::error::Error>> {
        let stats = generate_network_stats(state);
        write_stats(&stats, &stats_path)?;
        Ok(())
    };

    let aggregator_handle = thread_helper::aggregator_thread(
        packet_rx,
        initial_state,
        update_state,
        save_stats,
    );

    // 5. Watcher della cartella
    let watcher_result = thread_helper::folder_monitoring_thread(
        PathBuf::from(&config.watch_dir),
        watcher_file_tx,
    );

    let _watcher = match watcher_result {
        Ok(w) => {
            println!("Watcher avviato e in ascolto su: {}", config.watch_dir);
            w
        }
        Err(e) => {
            eprintln!("Errore nella creazione del watcher: {}", e);
            std::process::exit(1);
        }
    };

    // 6. Worker thread pool
    let worker_handles = thread_helper::generate_workers(
        config.parallelism,
        shared_job_rx,
        {
            let packet_tx = Arc::clone(&packet_tx);
            move |input, output| {
                match pcap_local_process(input.clone(), output) {
                    Ok(packets) => {
                        if let Err(e) = packet_tx.send(packets) {
                            eprintln!("Errore durante invio batch da {}: {}", input, e);
                        }
                        Ok(())
                    }
                    Err(e) => Err(format!("Errore nel file {}: {}", input, e)),
                }
            }
        },
    );

    // 7. Dispatcher dei job
    job_dispatcher::dispatch_jobs(watcher_file_rx, &job_tx, &config.output_dir);

    println!("Chiusura dei job e attesa dei worker...");
    drop(job_tx); // Chiude il canale: i worker termineranno

    for (i, handle) in worker_handles.into_iter().enumerate() {
        if let Err(e) = handle.join() {
            eprintln!("Errore nel join del worker {}: {:?}", i, e);
        }
    }

    // 8. Chiude l’ultimo sender → il receiver esce correttamente
    drop(packet_tx);
    aggregator_handle.join().unwrap();

    println!("Tutti i worker e l'aggregatore hanno terminato.");
}


fn pcap_local_process(input_path: String, output_path: String) -> Result<Vec<model::PacketData>, Box<dyn std::error::Error>> {
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

pub fn process_and_save_network_data(
    input: String,
    output: String,
    packet_tx: &Arc<Sender<Vec<PacketData>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    match pcap_local_process(input, output) {
        Ok(local_packets) => {
            let sender = Arc::clone(packet_tx);
            if let Err(e) = sender.send(local_packets) {
                eprintln!("Errore nell'invio del batch da: {}", e);
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Errore nel process_save_packets: {}", e);
            Err(e)
        }
    }
}



