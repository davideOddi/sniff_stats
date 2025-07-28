mod util;
mod service;
mod model;
mod network_capture;
mod pcap_helper;
mod stat_helper;
mod thread_helper;
mod job_dispatcher;

fn main() {
    let config: model::Config = service::load_config();
    println!("Configurazione caricata: {:?}", config.output_dir);
    service::monitor_network(config);
}
