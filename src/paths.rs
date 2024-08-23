use std::sync::RwLock;

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use lazy_static::lazy_static;
use crate::utils::UnwrapOrPanicJson;

lazy_static!{
    pub static ref BASE_PATH: RwLock<Utf8PathBuf> = RwLock::new(
        Utf8PathBuf::from(
            std::env::current_dir()
                .unwrap_or_panic_json()
                .join("data/state.json")
                .to_str()
                .context("Failed to convert path to string")
                .unwrap_or_panic_json()
                .to_string()
        )
    );
}

pub fn get_app_state_path() -> Result<Utf8PathBuf> {
    Ok(BASE_PATH.read().map_err(|_|{anyhow::anyhow!("Failed to get state path")})?.as_path().join("state.json"))
}

pub fn get_layers_path() -> Result<Utf8PathBuf> {
    Ok(BASE_PATH.read().map_err(|_|{anyhow::anyhow!("Failed to get layers path")})?.as_path().join("layers"))
}

pub fn get_layers_compressed_path() -> Result<Utf8PathBuf> {
    Ok(BASE_PATH.read().map_err(|_|{anyhow::anyhow!("Failed to get layers compressed path")})?.as_path().join("layers_compressed"))
}

pub fn get_docker_layers_path() -> Utf8PathBuf {
    Utf8PathBuf::from("/var/lib/docker/overlay2")
}

pub fn get_images_path() -> Result<Utf8PathBuf> {
    Ok(BASE_PATH.read().map_err(|_|{anyhow::anyhow!("Failed to get images path")})?.as_path().join("images"))
}