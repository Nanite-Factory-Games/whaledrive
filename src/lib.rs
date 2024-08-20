pub mod models;
pub mod utils;
pub mod commands;
pub mod application_state;

use std::sync::RwLock;

use anyhow::Context;
use camino::Utf8PathBuf;
use lazy_static::lazy_static;
use utils::UnwrapOrPanicJson;

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


