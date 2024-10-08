use std::fs;
use anyhow::{Context, Result};
use camino::Utf8PathBuf;

use crate::{
    application_state::{ApplicationState, Image, StateHandle}, docker_client::DockerClient, models::{
        input_models::*,
        output_models::{ImageInfoResult, ListImagesResult, MakeImageResult, PruneResult, RemoveImageResult},
        registry_models::Platform,
    }, paths::{get_images_path, get_layers_compressed_path}, utils::create_drive_image
};

/// Get the info about an image that will be downloaded
pub async fn image_info(args: ImageInfoArgs) -> Result<String> {
    let handle = StateHandle::new()?;
    let state = &handle.state;
    let platform = Platform {
        architecture: args.architecture,
        os: args.os
    };

    let client = DockerClient::new_with_auth(&args.image.name).await?;
    let manifests = client.get_manifests().await?;
    let manifest = manifests.get_manifest_for_platform(&platform).context("Manifest not found for platform")?;
    let oci_manifest = client.get_oci_manifest( &manifest.digest.as_str()).await?;
    let stored_digest = state.get_stored_image_digest(&args.image.name, &args.image.tag, &platform);
    let downloaded = stored_digest.is_some();
    let is_latest = matches!(stored_digest, Some(v) if v == oci_manifest.config.digest);
    Ok(serde_json::to_string_pretty(&ImageInfoResult {
        digest: oci_manifest.config.digest,
        downloaded,
        is_latest,
    })?)
}

/// Given a image name, tag and platform, create an ext4 file
pub async fn build_image(args: BuildImageArgs) -> Result<String> {
    let mut handle = StateHandle::new()?;
    let state = &mut handle.state;
    let result = build_image_remote(args, state).await?;
    Ok(serde_json::to_string_pretty(&result)?)
}

async fn build_image_remote(args: BuildImageArgs, state: &mut ApplicationState) -> Result<MakeImageResult> {    
    let layers_folder = get_layers_compressed_path()?;
    let images_folder = get_images_path()?;
    let platform = Platform {
        architecture: args.architecture,
        os: args.os
    };

    println!("building remote image for {}:{}", args.image.name, args.image.tag);

    let client = DockerClient::new_with_auth(&args.image.name).await?;
    let manifests = client.get_manifests().await?;
    let manifest = manifests.get_manifest_for_platform(&platform).context("Manifest not found for platform")?;
    let oci_manifest = client.get_oci_manifest(&manifest.digest.as_str()).await?;
    let stored_digest = state.get_stored_image_digest(&args.image.name, &args.image.tag, &platform);
    let mut file_path = images_folder.join(format!("{}.img", oci_manifest.config.digest)).to_string();
    let downloaded = stored_digest.is_some();
    let is_latest = !args.outfile.is_some() && matches!(&stored_digest, Some(v) if v == &oci_manifest.config.digest);
    // If the digest doesn't match, it means that we have to download new layers
    let size = if !is_latest {
        // Get layer digests
        let layers = oci_manifest.layers
            .iter()
            .map(|l|{l.digest.clone()})
            .collect::<Vec<String>>();
        // Download each layer
        client.download_layers_compressed(&layers).await?;
        let image_config = client.get_image_config(&oci_manifest.config.digest.as_str()).await?;
        let bootloader_path = image_config.config.labels.get("whaledrive.bootloader.path").context("Bootloader not found in image config")?;
        
        let image_directory = get_images_path()?;
        if let Some(outfile) = &args.outfile {
            file_path = outfile.to_string();
        }
        fs::create_dir_all(&image_directory)?;
        
        create_drive_image(
            &layers,
            layers_folder.as_std_path(),
            &bootloader_path,
            &Utf8PathBuf::from(&file_path)
        )?
    } else {
        let digest = stored_digest.context("Expected digest to exist")?;
        state.images.get(&digest).context(format!("Expected image {} to exist", digest))?.size
    };
    // Update state with info about the new image
    if !state.images.contains_key(&oci_manifest.config.digest) {
        state.images.insert(oci_manifest.config.digest.clone(), Image {
            name: args.image.name.clone(),
            tag: args.image.tag.clone(),
            platform: platform.clone(),
            size,
            layers: oci_manifest.layers.iter().map(|l|{l.digest.clone()}).collect::<Vec<String>>()
        });
    }
    state.set_stored_image_digest(&args.image.name, &args.image.tag, &platform, oci_manifest.config.digest.clone());
    Ok(MakeImageResult {
        digest: oci_manifest.config.digest,
        size,
        downloaded,
        file_path
    })
}

/// Clean all layers not associated with an existing image
pub fn prune() -> Result<String> {
    let mut handle = StateHandle::new()?;
    let state = &mut handle.state;
    
    let base_folder = std::env::current_dir()?.join("data");
    let active_digests: Vec<String> = state.tagged_images.values().cloned().collect();
    
    let inactive_layers: Vec<String> = state.layers.iter().filter(|l|{!active_digests.contains(l)}).cloned().collect();
    for layer in &inactive_layers {
        let path = base_folder.join(format!("layers/{}.tgz", layer));
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    Ok(serde_json::to_string_pretty(&PruneResult {
        layers: inactive_layers
    })?)
}

/// List all drive images that currently exist 
pub fn list_images() -> Result<String> {
    let handle = StateHandle::new()?;
    let state = &handle.state;
    
    let result = serde_json::to_string_pretty(&ListImagesResult {
        images: state.images.clone()
    })?;
    Ok(result)
}

pub fn remove_image(args: RemoveImageArgs) -> Result<String> {
    let mut handle = StateHandle::new()?;
    let state = &mut handle.state;

    let base_folder = std::env::current_dir()?.join("data");
    let platform = Platform {
        architecture: args.architecture.unwrap_or(String::from("amd64")),
        os: args.os.unwrap_or(String::from("linux"))
    };
    let digest = state.get_stored_image_digest(&args.image.name, &args.image.tag, &platform);
    let digest = digest.context(format!("Digest not found for image {}:{}", args.image.name, args.image.tag))?;
    let _ = fs::remove_file(base_folder.join(format!("images/{}.img", digest)));
    let tagged_name = format!(
        "{}:{}-{}:{}",
        args.image.name, args.image.tag,
        platform.os, platform.architecture
    );
    state.tagged_images.remove(&tagged_name);
    state.images.remove(&digest).context("Image not found")?;
    let active_digests: Vec<String> = state.images
        .iter()
        .map(|a|{a.1.layers.clone()})
        .flatten()
        .collect();
    let mut removed_layers = Vec::<String>::new();
    if args.prune {
        let inactive_layers: Vec<String> = state.layers.iter().filter(|l|{!active_digests.contains(l)}).cloned().collect();
        for layer in &inactive_layers {
            let path = base_folder.join(format!("layers/{}.tgz", layer));
            if path.exists() {
                fs::remove_file(path)?;
                removed_layers.push(layer.clone());
            }
        }
    }
    Ok(serde_json::to_string_pretty(&RemoveImageResult {
        digest,
        removed_layers
    })?)
}

/// Completely clean all images and layers stored
pub fn purge() {

}

