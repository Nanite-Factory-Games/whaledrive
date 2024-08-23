use std::collections::HashMap;

use serde::{Deserialize, Serialize};


#[derive(Deserialize, Debug)]
pub struct AuthResponse {
    pub token: String,
    pub access_token: String,
    pub expires_in: u64,
    pub issued_at: String
}

#[derive(Deserialize, Debug)]
pub struct Manifests {
    pub manifests: Option<Vec<Manifest>>,
    pub errors: Option<Vec<ManifestsError>>
}

impl Manifests {
    pub fn get_manifest_for_platform(&self, platform: &Platform) -> Option<Manifest> {
        if let Some(manifests) = &self.manifests {
            manifests.iter().find(|m| {
                m.platform == *platform
            }).cloned()
        } else {
            None
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ManifestsError {
    pub code: String,
    pub message: String,
    pub detail: Vec<ErrorDetail>
}

#[derive(Deserialize, Debug)]
pub struct ErrorDetail {
    #[serde(rename = "Type")]
    pub ty: String,
    #[serde(rename = "Class")]
    pub class: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Action")]
    pub action: String,


}

#[derive(Deserialize, Debug)]
pub struct ImageConfig {
    pub architecture: String,
    pub config: Config,
    pub created: String,
    pub history: Vec<History>,
    pub os: String,
    pub rootfs: RootFs,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(rename = "Env")]
    pub env: Vec<String>,
    #[serde(rename = "Entrypoint")]
    pub entrypoint: Vec<String>,
    #[serde(rename = "Cmd")]
    pub cmd: Vec<String>,
    #[serde(rename = "Labels")]
    pub labels: HashMap<String, String>,
    #[serde(rename = "ArgsEscaped")]
    pub args_escaped: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct History {
    pub created: String,
    pub created_by: String,
    pub comment: Option<String>,
    pub empty_layer: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RootFs {
    #[serde(rename = "type")]
    pub fs_type: String,
    pub diff_ids: Vec<String>,
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
    pub annotations: Option<HashMap<String, String>>
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