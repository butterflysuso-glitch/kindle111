pub mod audit;
pub mod convert;
pub mod dashboard;
pub mod epub;
pub mod incremental;
pub mod model;
pub mod pipeline;
pub mod quality;
pub mod review;
pub mod repair;

use std::{fs, path::Path};
use serde::{de::DeserializeOwned, Serialize};

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub fn write_new_json<T: Serialize>(path: &Path, data: &T) -> Result<(), String> {
    use std::io::Write;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut handle = fs::OpenOptions::new().write(true).create_new(true)
        .open(path).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(data).map_err(|e| e.to_string())?;
    handle.write_all(&bytes).map_err(|e| e.to_string())
}
