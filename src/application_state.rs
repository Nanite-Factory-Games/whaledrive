use std::{collections::HashMap, fs};

use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::{models::registry_models::Platform, utils::{get_app_state_path, UnwrapOrPanicJson}};




#[derive(Deserialize, Serialize, Debug)]
pub struct Layer {
    pub digest: String,
    pub size: u64
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Image {
    pub platform: Platform,
    pub name: String,
    pub tag: String,
    pub layers: Vec<String>,
    pub size: u64,
}

/// The json file that can be read from and written to
/// that stores the current images and layers
#[derive(Deserialize, Serialize, Debug)]
pub struct ApplicationState {
    /// Mapping of image:tag-os:arch to digest
    pub tagged_images: HashMap<String, String>,
    /// All the image files that are stored locally
    pub images: HashMap<String, Image>,
    /// All the layers stored locally
    pub layers: Vec<String>
}

impl ApplicationState {
    pub fn new() -> ApplicationState {
        ApplicationState {
            tagged_images: HashMap::new(),
            images: HashMap::new(),
            layers: Vec::new()
        }
    }

    /// Gets the digest for the stored image with the provided
    /// name, tag and platform
    pub fn get_stored_image_digest(&self, name: &str, tag: &str, platform: &Platform) -> Option<String> {
        let key = format!("{name}:{tag}-{}:{}", platform.os, platform.architecture);
        self.tagged_images.get(&key).and_then(|m|{Some(m.clone())})
    }

    /// Gets the digest for the stored image with the provided
    /// name, tag and platform
    pub fn set_stored_image_digest(&mut self, name: &str, tag: &str, platform: &Platform, digest: String) {
        let key = format!("{name}:{tag}-{}:{}", platform.os, platform.architecture);
        self.tagged_images.insert(key, digest);
    }

    /// Gets the digest for the stored image with the provided
    /// name, tag and platform
    pub fn get_stored_image(&self, name: &str, tag: &str, platform: &Platform) -> Option<Image> {
        let digest = self.get_stored_image_digest(name, tag, platform)?;
        self.images.get(&digest).map(|i|{i.clone()})
    }
}

pub struct StateHandle {
    pub state: ApplicationState
}

/// We use a state handle to automatically read and write the state file
impl StateHandle {
    pub fn new() -> Result<StateHandle> {
        let state = serde_json::from_str(
            &std::fs::read_to_string(&get_app_state_path()?)?
        )?;
        Ok(StateHandle {state})
    }
}

impl Drop for StateHandle { 
    fn drop(&mut self) {
        let res = (|| -> Result<()> {
            let json = serde_json::to_string_pretty(&self.state)?;
            fs::write(
               get_app_state_path()?,
                json.as_bytes()
            )?;
            Ok(())
        })();
        res.unwrap_or_panic_json();
    }
}