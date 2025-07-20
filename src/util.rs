use std::{fs::File, path};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub watch_dir: String,
    pub output_dir: String,
    pub parallelism: i8,
}

/* 
pub fn read_from_json(file_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let file: File = std::fs::File::open(file_path)?;
    let config: Config = serde_json::from_reader(file)?;
    return Ok(config)
}
*/

pub fn read_json_file_as<T, P>(file_path: P) -> Result<T, Box<dyn std::error::Error>>
    where
        T: serde::de::DeserializeOwned,
        P: AsRef<crate::util::path::Path>, 
{
    let file: File = File::open(file_path)?; 
    let json_data: T = serde_json::from_reader(file)?;
    return Ok(json_data);
}
