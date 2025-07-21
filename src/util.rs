use std::{fs::File, path};

pub fn read_json_file_as<T, P>(file_path: P) -> Result<T, Box<dyn std::error::Error>>
    where
        T: serde::de::DeserializeOwned,
        P: AsRef<crate::util::path::Path>, 
{
    let file: File = File::open(file_path)?; 
    let json_data: T = serde_json::from_reader(file)?;
    return Ok(json_data);
}
