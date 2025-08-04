use crate::job_dispatcher;
use crate::model;
use crate::model::NetworkStats;
use model::Config;
use crate::util;
use crate::network_capture;
use std::error::Error;
use std::path::PathBuf;
use crate::thread_helper;
use crate::model::PacketData;
use std::sync::{mpsc::{channel}, Arc, Mutex};
use crate::thread::thread_factory::{create_thread, ThreadType};

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
    // canale di invio per i file (notified) che dovranno essere procesati dai worker
    let (watcher_file_tx, watcher_file_rx) = channel::<PathBuf>();
    // canale di distribuzione dei job con i file path da processare
    let (job_tx, job_rx) = channel::<(String, String)>();
    // arc mutex PER FORZA per condividere il receiver tra i worker
    let shared_job_rx = Arc::new(Mutex::new(job_rx));
    // canale per inviare i pacchetti processati all'aggregatore
    let (packet_tx, packet_rx) = channel::<Vec<PacketData>>();
    let packet_tx = Arc::new(packet_tx);

    // total_stats forse nel file di properties?
    let stats_path = format!("{}/total_stats.json", config.output_dir);
    let target_dir = PathBuf::from(stats_path);

    let save_stats = create_save_stats_fn(target_dir.clone());

    let aggregator = create_thread(ThreadType::Aggregator {
    name: "Aggregatore statistiche".to_string(),
    packet_rx,
    save_stats,
    });


    // Watcher della cartella con notify
    let watcher = create_thread(ThreadType::Watcher {
    name: "Osservatore Cartella".to_string(),
    path: PathBuf::from(config.watch_dir.clone()),
    sender: watcher_file_tx,
    });

    // serve per mantenere il watcher attivo
    let watcher_ancor = watcher;

    // creo tanti worker quanto config.parallelism 
    //e ogni worker riceve un job dal canale condiviso (pcap_local_process)
    let worker_handles: Vec<std::thread::JoinHandle<()>> = thread_helper::generate_workers(
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

    // smista i file ricevuti dal watcher ai worker dopo avergli assegnato il job
    job_dispatcher::dispatch_jobs(watcher_file_rx, &job_tx, &config.output_dir);

    println!("Chiusura dei job e attesa dei worker...");
    drop(job_tx); // Chiude il canale: i worker termineranno

    //attendo la fine dei processi dei worker
    for (i, handle) in worker_handles.into_iter().enumerate() {
        if let Err(e) = handle.join() {
            eprintln!("Errore nel join del worker {}: {:?}", i, e);
        }
    }

    // Chiude l’ultimo sender e aspetta che l'aggragator_thread termini
    drop(packet_tx);
    aggregator.join();

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

pub fn write_stats(aggregated_stats: &NetworkStats, output_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    util::update_file(output_path, aggregated_stats)
        .map_err(|e| {
            eprintln!("Errore nell'aggiornamento del file delle statistiche: {}", e);
            e.into()
        })
}

pub fn create_save_stats_fn(
    output: PathBuf,
) -> Box<dyn Fn(&Vec<PacketData>) -> Result<(), Box<dyn Error>> + Send + Sync + 'static> {
    Box::new(move |state: &Vec<PacketData>| -> Result<(), Box<dyn Error>> {
        let stats = generate_network_stats(state);
        write_stats(&stats, &output)?;
        Ok(())
    })
}



