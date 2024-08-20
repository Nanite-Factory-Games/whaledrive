use std::collections::HashMap;

use serde::{Deserialize, Serialize};


#[derive(Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub access_token: String,
    pub expires_in: u64,
    pub issued_at: String
}

#[derive(Deserialize, Debug)]
pub struct Manifests {
    pub manifests: Vec<Manifest>
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Platform {
    pub architecture: String,
    pub os: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Manifest {
    pub digest: String,
    pub platform: Platform
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub media_type: String,
    pub digest: String,
    pub size: u64
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OCIManifest {
    pub schema_version: u64,
    pub media_type: String,
    pub config: OCIManifestConfig,
    pub layers: Vec<Layer>,
    pub annotations: HashMap<String, String>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OCIManifestConfig {
    pub media_type: String,
    pub digest: String,
    pub size: u64
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct LocalImageManifest {
    pub config: String,
    pub repo_tags: Vec<String>,
    pub layers: Vec<String>
}

impl PartialEq for Platform {
    fn eq(&self, other: &Self) -> bool {
        self.architecture == other.architecture && self.os == other.os
    }
}