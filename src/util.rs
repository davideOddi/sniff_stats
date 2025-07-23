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

pub fn write_json_file<P, T>(file_path: P, data: &T) -> Result<(), Box<dyn std::error::Error>>
    where
        P: AsRef<crate::util::path::Path>,
        T: serde::Serialize,
{
    let file: File = File::create(file_path)?;
    serde_json::to_writer_pretty(file, data)?;
    return Ok(());
}
