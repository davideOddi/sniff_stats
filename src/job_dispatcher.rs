use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;

pub fn dispatch_jobs(
    watcher_rx: Receiver<PathBuf>,
    job_senders: Vec<Sender<(String, String)>>,
    output_dir: &str,
) {
    let mut seen_files = HashSet::new();
    let mut index = 0;

    for pcap_path in watcher_rx {
        if pcap_path.extension().and_then(|e| e.to_str()) == Some("pcap") {
            let path_str = pcap_path.to_string_lossy().to_string();

            if seen_files.insert(path_str.clone()) {
                if let Some((input, output)) = path_builder(&pcap_path, output_dir) {
                    let sender = &job_senders[index % job_senders.len()];
                    if let Err(e) = sender.send((input, output)) {
                        eprintln!("[dispatcher] Errore invio job: {}", e);
                    }
                    index += 1;
                } else {
                    eprintln!("[dispatcher] Errore nella costruzione dei path per file: {:?}", pcap_path);
                }
            }
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
