use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use notify::{RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher};
use crate::model::PacketData;
use crate::thread::model::{Thread, ThreadWatcher, ThreadWithState, ThreadWorker};

pub enum ThreadHandle {
    Watcher(ThreadWatcher),
    Aggregator(ThreadWithState<Vec<PacketData>>),
    Worker(ThreadWorker),
}

pub enum ThreadType {
    Watcher {
        name: String,
        path: PathBuf,
        sender: Sender<PathBuf>,
    },
    Aggregator {
        name: String,
        packet_rx: Receiver<Vec<PacketData>>,
        save_stats: Box<
            dyn Fn(&Vec<PacketData>) -> Result<(), Box<dyn Error>> + Send + Sync + 'static,
        >,
    },
    Worker {
        name: String,
        job_rx: Arc<Mutex<Receiver<(String, String)>>>,
        packet_tx: Arc<Sender<Vec<PacketData>>>,
        worker_fn:
            Arc<dyn Fn(String, String) -> Result<Vec<PacketData>, Box<dyn Error>> + Send + Sync + 'static>,
    },
}

impl ThreadHandle {
    pub fn join(self) {
        match self {
            ThreadHandle::Watcher(w) => w.join(),
            ThreadHandle::Aggregator(a) => a.join(),
            ThreadHandle::Worker(w) => w.join(),
        }
    }
}

fn create_watcher(
    name: String,
    path: PathBuf,
    sender: Sender<PathBuf>,
) -> NotifyResult<ThreadWatcher> {
    let (notify_tx, notify_rx) = std::sync::mpsc::channel();

    let mut watcher: RecommendedWatcher = notify::recommended_watcher(notify_tx)?;
    watcher.watch(&path, RecursiveMode::NonRecursive)?;

    let name_cloned = name.clone();
    let thread_job = move || {
        println!("[{}] In ascolto su {}", name_cloned, path.display());
        for event_result in notify_rx {
            if let Ok(event) = event_result {
                if matches!(event.kind, notify::EventKind::Create(_))
                    || matches!(event.kind, notify::EventKind::Modify(_))
                {
                    for path in event.paths {
                        if path.extension().and_then(|e| e.to_str()) == Some("pcap") {
                            println!("[{}] Nuovo file pcap: {:?}", name_cloned, path);
                            if let Err(e) = sender.send(path.clone()) {
                                eprintln!("[{}] Errore invio: {}", name_cloned, e);
                            }
                        }
                    }
                }
            } else {
                eprintln!("[{}] Errore evento: {:?}", name_cloned, event_result);
            }
        }
    };

    let thread = Thread::new(&name, thread_job);

    Ok(ThreadWatcher {
        base: thread,
        _watcher: watcher,
    })
}

pub fn create_stats_aggregator(
    name: &str,
    packet_rx: Receiver<Vec<PacketData>>,
    save_stats: Box<dyn Fn(&Vec<PacketData>) -> Result<(), Box<dyn Error>> + Send + Sync + 'static>,
) -> ThreadWithState<Vec<PacketData>> {
    let name = name.to_string();

    ThreadWithState::new(
        &name,
        Vec::new(),
        packet_rx,
        move |state: &mut Vec<PacketData>, batch: Vec<PacketData>| {
            state.extend(batch);
            if let Err(e) = save_stats(state) {
                eprintln!("Errore salvataggio stats: {}", e);
            }
        },
    )
}

pub fn create_thread(thread_type: ThreadType) -> ThreadHandle {
    match thread_type {
        ThreadType::Watcher { name, path, sender } => match create_watcher(name, path, sender) {
            Ok(watcher) => {
                println!("Thread {} Avviato", watcher.base.name);
                ThreadHandle::Watcher(watcher)
            }
            Err(e) => {
                eprintln!("Errore nella creazione del watcher: {}", e);
                std::process::exit(1);
            }
        },
        ThreadType::Aggregator {
            name,
            packet_rx,
            save_stats,
        } => {
            let aggregator = create_stats_aggregator(&name, packet_rx, save_stats);
            ThreadHandle::Aggregator(aggregator)
        }
        ThreadType::Worker {
            name,
            job_rx,
            packet_tx,
            worker_fn,
        } => {
            let worker = ThreadWorker::new::<Vec<PacketData>, _>(
                &name,
                job_rx,
                packet_tx,
                worker_fn,
            );
            ThreadHandle::Worker(worker)
        }
    }
}
