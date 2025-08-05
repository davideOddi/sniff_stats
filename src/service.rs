use crate::job_dispatcher;
use crate::model;
use crate::model::NetworkStats;
use crate::thread::thread_factory::create_thread;
use crate::thread::thread_factory::ThreadType;
use model::Config;
use crate::util;
use crate::network_capture;
use std::error::Error;
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

    let watcher_handle = create_thread(ThreadType::Watcher {
        name: "watcher".to_string(),
        path: PathBuf::from(&config.watch_dir),
        sender: watcher_file_tx,
    });

    // 2. Dispatcher → manda job (input, output) ai worker
    let (job_tx, job_rx) = channel::<(String, String)>();
    let shared_job_rx = Arc::new(Mutex::new(job_rx));

    // 3. Worker → manda batch di pacchetti all’aggregatore
    let (packet_tx, packet_rx) = channel::<Vec<PacketData>>();
    let packet_tx = Arc::new(packet_tx);

    // 4. Aggregatore
    let stats_path = format!("{}/total_stats.json", config.output_dir);
    let save_stats = move |state: &Vec<PacketData>| -> Result<(), Box<dyn Error>> {
        let stats = crate::stat_helper::generate_stats(state);
        crate::util::update_file(&stats_path, &stats)?;
        Ok(())
    };

    let aggregator_handle = create_thread(ThreadType::Aggregator {
        name: "aggregator".to_string(),
        packet_rx,
        save_stats: Box::new(save_stats),
    });

    // 5. Worker pool
    let mut worker_handles = Vec::new();

    for i in 0..config.parallelism {
        let worker_name = format!("worker-{}", i);
        let job_rx_clone = Arc::clone(&shared_job_rx);
        let packet_tx_clone = Arc::clone(&packet_tx);

        let worker_fn = Arc::new(|input: String, output: String| -> Result<Vec<PacketData>, Box<dyn Error>> {
            let packets = crate::network_capture::pcap_reader(&input)?;
            let stats = crate::stat_helper::generate_stats(&packets);
            crate::util::write_json_file(&output, &stats)?;
            Ok(packets)
        });

        let worker = create_thread(ThreadType::Worker {
            name: worker_name,
            job_rx: job_rx_clone,
            packet_tx: packet_tx_clone,
            worker_fn,
        });

        worker_handles.push(worker);
    }

    // 6. Dispatcher dei job
    job_dispatcher::dispatch_jobs(watcher_file_rx, &job_tx, &config.output_dir);

    // 7. Cleanup
    println!("Chiusura dei job e attesa dei worker...");
    drop(job_tx);
    drop(packet_tx);

    for handle in worker_handles {
        handle.join();
    }

    aggregator_handle.join();
    watcher_handle.join();

    println!("Tutti i worker e l'aggregatore hanno terminato.");
}


fn pcap_local_process(input_path: String, output_path: String) -> Result<Vec<model::PacketData>, Box<dyn Error>> {
    let data_packets: Vec<model::PacketData> = retrieve_data_packets(&input_path)?;
    let stats: model::NetworkStats = generate_network_stats(&data_packets);
    save_stats_to_file(&stats, &output_path)?;
    Ok(data_packets)
}

fn save_stats_to_file(stats: &model::NetworkStats, file_path: &str) -> Result<(), Box<dyn Error>> {
    util::write_json_file(file_path, stats)
        .map(|_| println!("Statistiche salvate in {}", file_path))
        .map_err(|e| {
            eprintln!("Errore nel salvataggio delle statistiche: {} per input file -> {}", e, file_path);
            e.into()
        })
}

fn retrieve_data_packets(file_path: &str) -> Result<Vec<model::PacketData>, Box<dyn Error>> {
    let packets = network_capture::pcap_reader(file_path)?;
    Ok(packets)
}

fn generate_network_stats(data_packets: &Vec<model::PacketData>) -> model::NetworkStats {
    return crate::stat_helper::generate_stats(data_packets);
}

fn write_stats(aggregated_stats: &NetworkStats, output_path: &str) -> Result<(), Box<dyn Error>> {
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
