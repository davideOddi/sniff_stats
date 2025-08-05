use std::{fs::{File, rename}, path::{Path}};
use std::{fs, io};


pub fn read_json_file_as<T, P>(file_path: P) -> Result<T, Box<dyn std::error::Error>>
    where
        T: serde::de::DeserializeOwned,
        P: AsRef<std::path::Path>, 
{
    let file: File = File::open(file_path)?; 
    let json_data: T = serde_json::from_reader(file)?;
    return Ok(json_data);
}

pub fn write_json_file<P, T>(file_path: P, data: &T) -> Result<(), Box<dyn std::error::Error>>
    where
        P: AsRef<std::path::Path>,
        T: serde::Serialize,
{
    let file: File = File::create(file_path)?;
    serde_json::to_writer_pretty(file, data)?;
    return Ok(());
}

pub fn update_file<P, T>(file_path: P, data: &T) -> Result<(), Box<dyn std::error::Error>>
where
    P: AsRef<Path>,
    T: serde::Serialize,
{
    let path = file_path.as_ref();
    let old_path = path.with_extension("old");
    if path.exists() {
        
        match rename(path, &old_path) {
            Ok(_) => println!("File esistente rinominato in: {}", old_path.display()),
            Err(e) => {
                return Err(Box::new(io::Error::new(io::ErrorKind::Other, format!("Errore nel rinominare il file: {}", e))))
            },
        }
    }

    match write_json_file(path, data) {
        Ok(_) => {
            if old_path.exists() {
                fs::remove_file(&old_path)?;
            }
            Ok(())
        },
        Err(e) => {
            if old_path.exists() {
                rename(&old_path, path)?;
            }
            Err(e)
        },
    }
}