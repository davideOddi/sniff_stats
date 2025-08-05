use crate::model;
use crate::thread::factory::{ThreadType, ThreadHandle, create_thread};
use model::{Config, PacketData};
use crate::util;
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc::{channel}, Arc, Mutex};

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
    // Inizializzazione canali
    let (watcher_file_tx, watcher_file_rx) = channel::<PathBuf>();
    let (job_tx, job_rx) = channel::<(String, String)>();
    let shared_job_rx = Arc::new(Mutex::new(job_rx));
    let (packet_tx, packet_rx) = channel::<Vec<PacketData>>();
    let packet_tx = Arc::new(packet_tx);

    let watcher_handle = create_watcher(&config.watch_dir, watcher_file_tx);

    let aggregator_handle = create_aggregator(&config.output_dir, packet_rx);

    // Definizione specifica la funzine dei worker con I/O
    let worker_fn = Arc::new(process_local_pcap);
    let worker_handles = 
        generate_workers_with_assignment(config.parallelism as usize, shared_job_rx, packet_tx.clone(), worker_fn);

    // Smistaggio paths nei canali 
    crate::job_dispatcher::dispatch_jobs(watcher_file_rx, &job_tx, &config.output_dir);

    // Cleanup 
    drop(job_tx);
    drop(packet_tx);
    wait_for_workers(worker_handles);
    aggregator_handle.join();
    watcher_handle.join();

    println!("Tutti i thread hanno terminato.");
}


fn create_watcher(watch_dir: &str, sender: Sender<PathBuf>) -> ThreadHandle {
    create_thread(ThreadType::Watcher {
        name: "watcher".to_string(),
        path: PathBuf::from(watch_dir),
        sender,
    })
}

fn create_aggregator(
    output_dir: &str,
    packet_rx: Receiver<Vec<PacketData>>,
) -> ThreadHandle {
    let stats_path = format!("{}/total_stats.json", output_dir);
    let save_stats = move |state: &Vec<PacketData>| -> Result<(), Box<dyn Error>> {
        let stats = crate::stat_helper::generate_stats(state);
        crate::util::update_file(&stats_path, &stats)?;
        Ok(())
    };

    create_thread(ThreadType::Aggregator {
        name: "aggregator".to_string(),
        packet_rx,
        save_stats: Box::new(save_stats),
    })
}

fn generate_workers_with_assignment(
    count: usize,
    shared_job_rx: Arc<Mutex<Receiver<(String, String)>>>,
    packet_tx: Arc<Sender<Vec<PacketData>>>,
    worker_fn: Arc<dyn Fn(String, String) -> Result<Vec<PacketData>, Box<dyn Error>> + Send + Sync>,
) -> Vec<ThreadHandle> {
    let mut handles = Vec::with_capacity(count);
    for i in 0..count {
        let name = format!("worker-{}", i);
        let job_rx = Arc::clone(&shared_job_rx);
        let packet_tx = Arc::clone(&packet_tx);
        let worker_fn = Arc::clone(&worker_fn);

        let worker = create_thread(ThreadType::Worker {
            name,
            job_rx,
            packet_tx,
            worker_fn,
        });

        handles.push(worker);
    }
    handles
}

fn wait_for_workers(workers: Vec<ThreadHandle>) {
    for (i, handle) in workers.into_iter().enumerate() {
        if let ThreadHandle::Worker(worker) = handle {
            worker.join();
        } else {
            eprintln!("Thread {} non è un Worker!", i);
        }
    }
}

fn process_local_pcap(input: String, output: String) -> Result<Vec<PacketData>, Box<dyn Error>> {
    let packets = crate::network_capture::pcap_reader(&input)?;
    let stats = crate::stat_helper::generate_stats(&packets);
    crate::util::write_json_file(&output, &stats)?;
    Ok(packets)
}
