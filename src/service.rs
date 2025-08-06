use crate::model::{Config, PacketData};
use crate::thread::factory::{create_thread, ThreadHandle, ThreadType};
use crate::util;
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

pub fn load_config() -> Config {
    const CONFIG_PATH: &str = "properties.json";
    match util::read_json_file_as(CONFIG_PATH) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Errore nel caricamento della configurazione: {}", e);
            std::process::exit(1);
        }
    }
}

pub fn monitor_network(config: Config) {
    let (watcher_tx, watcher_rx) = channel::<PathBuf>();

    let (packet_tx, packet_rx) = channel::<Vec<PacketData>>();
    let packet_tx = Arc::new(packet_tx);

    let worker_fn: Arc<fn(String, String) -> Result<Vec<PacketData>, Box<dyn Error>>> =
        Arc::new(process_local_pcap);

    let (worker_handles, job_senders) =
        generate_workers_with_assignment(config.parallelism as usize, packet_tx.clone(), worker_fn);

    let watcher_handle = create_watcher(&config.watch_dir, watcher_tx);
    let aggregator_handle = create_aggregator(&config.output_dir, packet_rx);

    crate::job_dispatcher::dispatch_jobs(watcher_rx, job_senders, &config.output_dir);

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

fn create_aggregator(output_dir: &str, packet_rx: Receiver<Vec<PacketData>>) -> ThreadHandle {
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
    packet_tx: Arc<Sender<Vec<PacketData>>>,
    worker_fn: Arc<dyn Fn(String, String) -> Result<Vec<PacketData>, Box<dyn Error>> + Send + Sync>,
) -> (Vec<ThreadHandle>, Vec<Sender<(String, String)>>) {
    let mut handles = Vec::with_capacity(count);
    let mut job_senders = Vec::with_capacity(count);

    for i in 0..count {
        let (job_tx, job_rx) = channel::<(String, String)>();
        let name = format!("worker-{}", i);

        let worker = create_thread(ThreadType::Worker {
            name,
            job_rx: Arc::new(std::sync::Mutex::new(job_rx)),
            packet_tx: Arc::clone(&packet_tx),
            worker_fn: Arc::clone(&worker_fn),
        });

        handles.push(worker);
        job_senders.push(job_tx);
    }

    (handles, job_senders)
}

fn wait_for_workers(workers: Vec<ThreadHandle>) {
    for (i, handle) in workers.into_iter().enumerate() {
        if let ThreadHandle::Worker(worker) = handle {
            worker.join();
        }
    }
}

fn process_local_pcap(input: String, output: String) -> Result<Vec<PacketData>, Box<dyn Error>> {
    let packets = crate::network_capture::pcap_reader(&input)?;
    let stats = crate::stat_helper::generate_stats(&packets);
    crate::util::write_json_file(&output, &stats)?;
    Ok(packets)
}
