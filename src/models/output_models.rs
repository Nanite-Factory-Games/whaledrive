use std::collections::HashMap;

use serde::Serialize;

use crate::application_state::Image;


#[derive(Serialize)]
pub struct ImageInfoResult {
    /// Digest of the resultant image
    pub digest: String,
    /// Is this already downloaded
    pub downloaded: bool,
    /// Is the version that is downloaded the same sha as the remote one?
    pub is_latest: bool
}

#[derive(Serialize)]
pub struct MakeImageResult {
    /// Digest of the resultant image
    pub digest: String,
    /// Size of the resultant image in bytes
    pub size: u64,
    /// Is this already downloaded
    pub downloaded: bool,
    /// File path of the image generated
    pub file_path: String,
}

#[derive(Serialize)]
pub struct ListImagesResult {
    /// List of images
    pub images: HashMap<String, Image>
}

#[derive(Serialize)]
pub struct RemoveImageResult {
    /// Digest of the resultant image
    pub digest: String,
    /// layers that were removed
    pub removed_layers: Vec<String>,
}

#[derive(Serialize)]
pub struct PruneResult {
    /// All the layers that were pruned
    pub layers: Vec<String>
}