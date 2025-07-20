mod util;
mod service;
use util::Config;

fn main() {
    let config: Config = service::load_config();
    println!("Configurazione caricata: {:?}", config.output_dir);
}
