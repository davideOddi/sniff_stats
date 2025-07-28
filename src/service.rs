use crate::model;
use model::Config;
use crate::util;
use crate::network_capture;
use std::path::PathBuf;
use std::thread;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::{Sender};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};


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

    // per creare tanti worker devo PER FORZA usare un Arc<Mutex<Receiver>>
    let shared_job_rx = Arc::new(Mutex::new(job_rx));


    let watcher_result = folder_monitoring_thread(
        PathBuf::from(&config.watch_dir),
        watcher_file_tx
    );

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

    let mut worker_handles = Vec::new(); 
    for i in 0..config.parallelism {
        // clono tanti worker quanto specificato in config.parallelism
        // Ogni worker avrà il suo canale di ricezione dei job
        let worker_job_rx_shared_clone = Arc::clone(&shared_job_rx);
 

        let handle = thread::spawn(move || {
            println!("Worker {} avviato.", i);
            loop { 
                let job = {
                    let locked_rx = worker_job_rx_shared_clone.lock().unwrap();
                    locked_rx.recv() 
                };

                match job {
                    Ok((input_path, output_path)) => {
                        println!("[Worker {}] Ricevuto job per: {}", i, input_path);
                        match work_process(input_path.clone(), output_path.clone()) {
                            Ok(_packets) => {
                            }
                            Err(e) => {
                                eprintln!("[Worker {}] Errore nell'elaborazione del file {}: {}", i, input_path, e);
                            }
                        }
                    }
                    Err(std::sync::mpsc::RecvError) => {
                        println!("Worker {} - Canale job chiuso, terminazione.", i);
                        break;
                    }
                }
            }
            println!("Worker {} terminato.", i);
        });
        worker_handles.push(handle); 
    }
    let mut processed_files: HashSet<PathBuf> = HashSet::new();

    // Logica di invio: riceve file dal watcher e li invia ai worker
    // solo per file non processatu
    for pcap_path in watcher_file_rx {
        if processed_files.contains(&pcap_path) {
            println!("File già in coda o processato: {:?}", pcap_path);
            continue;
        }
        processed_files.insert(pcap_path.clone());
        println!("Aggiunto a Set, dimensione: {}", processed_files.len());
        println!("File ricevuto dal watcher: {:?}", pcap_path);

        let input_path_str = pcap_path
            .to_str()
            .map(|s| s.to_string()) 
            .unwrap_or_else(|| {
                eprintln!("Errore: Impossibile convertire PathBuf in stringa per {:?}", pcap_path);
                return String::new();
            });

        if input_path_str.is_empty() {
            continue;
        }

        let file_name = pcap_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown_file"); 

        let output_path_buf = PathBuf::from(&config.output_dir)
            .join(format!("{}.json", file_name));

        let output_path_str = output_path_buf
            .to_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                eprintln!("Errore: Impossibile creare percorso di output per {:?}", output_path_buf);
                return String::new();
            });

        if output_path_str.is_empty() {
            continue;
        }

        // Invia il job al pool
        if let Err(e) = job_tx.send((input_path_str, output_path_str)) {
            eprintln!("Errore nell'invio del job al worker: {}", e);
        }
    }


    drop(job_tx);
    println!("Canale job_tx chiuso. Attendendo che i worker terminino i job rimanenti...");

    for (i, handle) in worker_handles.into_iter().enumerate() {
        if let Err(e) = handle.join() {
            eprintln!("Errore nel join del worker {}: {:?}", i, e);
        }
    }
    println!("Tutti i worker hanno terminato.");
    println!("Monitoraggio della rete terminato.");
}


fn work_process(input_path: String, output_path: String) -> Result<Vec<model::PacketData>, Box<dyn std::error::Error>> {
    let data_packets: Vec<model::PacketData> = retrieve_data_packets(&input_path)?;
    let stats: model::NetworkStats = generate_network_stats(&data_packets);
    save_stats_to_file(&stats, &output_path);
    Ok(data_packets)
}

fn save_stats_to_file(stats: &model::NetworkStats, file_path: &str) {
    match util::write_json_file(file_path, stats) {
        Ok(_) => println!("Statistiche salvate in {}", file_path),
        Err(e) => 
            eprintln!("Errore nel salvataggio delle statistiche: {} per input file -> {}" 
            , e, file_path),
    }
}

fn retrieve_data_packets(file_path: &str) -> Result<Vec<model::PacketData>, Box<dyn std::error::Error>> {
    let packets = network_capture::pcap_reader(file_path)?;
    Ok(packets)
}

fn generate_network_stats(data_packets: &Vec<model::PacketData>) -> model::NetworkStats {
    return crate::stat_helper::generate_stats(data_packets);
}

fn folder_monitoring_thread(path: PathBuf, file_sender: Sender<PathBuf>) 
-> notify::Result<RecommendedWatcher>  {
     let (notify_tx, notify_rx) = std::sync::mpsc::channel();

    // mantengo il watcher in un thread separato e attivo
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(notify_tx)?;
    watcher.watch(&path, RecursiveMode::NonRecursive)?;

    println!("In ascolto sulla cartella: {}", path.display());

    thread::spawn(move || {
        for event_result in notify_rx {
            match event_result {
                Ok(event) => {
                    if matches!(event.kind, notify::EventKind::Create(_)) ||
                    // modify event per testarare .pcap predefiniti
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
