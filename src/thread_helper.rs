use std::path::PathBuf;
use std::sync::mpsc::Sender;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;
use std::thread::{JoinHandle};
use std::error::Error;


pub fn folder_monitoring_thread(path: PathBuf, file_sender: Sender<PathBuf>) 
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


pub fn generate_workers<F>(
    num_workers: i8,
    receiver: Arc<Mutex<Receiver<(String, String)>>>,
    process: F,
) -> Vec<thread::JoinHandle<()>>
where
    F: Fn(String, String) -> Result<(), String> + Send + Sync + 'static,
{
    let process = Arc::new(process);
    let mut worker_handles = Vec::new();

    for i in 0..num_workers {
        let worker_receiver_clone = Arc::clone(&receiver);
        let process_clone = Arc::clone(&process);

        let handle = thread::spawn(move || {
            println!("Worker {} avviato.", i);
            loop {
                let job = {
                    let locked_rx = worker_receiver_clone.lock().unwrap();
                    locked_rx.recv()
                };

                match job {
                    Ok((input_path, output_path)) => {
                        println!("[Worker {}] Ricevuto job per: {}", i, input_path);
                        match process_clone(input_path.clone(), output_path.clone()) {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!(
                                    "[Worker {}] Errore nel file {}: {}",
                                    i, input_path, e
                                );
                            }
                        }
                    }
                    Err(_) => {
                        println!("Worker {} - Canale chiuso, uscita.", i);
                        break;
                    }
                }
            }
            println!("Worker {} terminato.", i);
        });

        worker_handles.push(handle);
    }

    worker_handles
}

pub fn aggregator_thread<T, State, UpdateFn, SaveFn>(
    receiver: Receiver<T>,
    mut state: State,
    mut update_state: UpdateFn,
    mut save_stats: SaveFn,
) -> JoinHandle<()>
where
    T: Send + 'static,
    State: Send + 'static,
    UpdateFn: FnMut(&mut State, T) + Send + 'static,
    SaveFn: FnMut(&State) -> Result<(), Box<dyn Error>> + Send + 'static,
{
    thread::spawn(move || {
        while let Ok(item) = receiver.recv() {
            update_state(&mut state, item);

            if let Err(e) = save_stats(&state) {
                eprintln!("Errore nel salvataggio delle statistiche: {}", e);
            }
        }
        println!("Thread aggregatore terminato.");
    })
}


