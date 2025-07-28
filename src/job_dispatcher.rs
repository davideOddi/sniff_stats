use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

pub fn dispatch_jobs(
    watcher_file_rx: std::sync::mpsc::Receiver<PathBuf>,
    job_tx: &Sender<(String, String)>,
    output_dir: &str,
) {
    let mut processed_files = HashSet::new();

    // Logica di invio: riceve file dal watcher e li invia ai worker
    // solo per file non processatu
    for pcap_path in watcher_file_rx {
        if !processed_files.insert(pcap_path.clone()) {
            println!("File già in coda o processato: {:?}", pcap_path);
            continue;
        }

        match path_builder(&pcap_path, output_dir) {
            Some((input_path, output_path)) => {
                if let Err(e) = job_tx.send((input_path, output_path)) {
                    eprintln!("Errore nell'invio del job al worker: {}", e);
                }
            }
            None => continue,
        }
    }
}

fn path_builder(pcap_path: &Path, output_dir: &str) -> Option<(String, String)> {
    let input_path = pcap_path.to_str()?.to_string();
    let file_name = pcap_path.file_name()?.to_str().unwrap_or("unknown_file");
    let output_path = PathBuf::from(output_dir)
        .join(format!("{}.json", file_name))
        .to_str()?
        .to_string();

    Some((input_path, output_path))
}
