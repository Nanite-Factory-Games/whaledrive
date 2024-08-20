use core::str;
use std::{fmt::Display, fs::{self, File}, io, path::Path, process::{Command, Output, Stdio}};
use anyhow::{anyhow, bail, Result};
use camino::Utf8PathBuf;
use flate2::read::GzDecoder;
use serde_json::json;
use tar::Archive;
use tempfile::TempDir;
use fs_extra::file::{move_file, CopyOptions};


use reqwest::Client;
use reqwest::header::ACCEPT;
use tokio::io::AsyncWriteExt;
use tokio::fs::File as TokioFile;

use crate::{models::registry_models::{AuthResponse, Manifest, Manifests, OCIManifest, Platform}, BASE_PATH};

const REGISTRY_URL: &str = "https://registry-1.docker.io";
const AUTH_URL: &str = "https://auth.docker.io";
const SVC_URL: &str = "registry.docker.io";

pub struct DockerClient {
    client: Client
}

impl DockerClient {

    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new()
        }
    }

    pub async fn get_auth_for_image(&self, image: &str) -> Result<AuthResponse> {
        let url = format!("{}/token?service={}&scope=repository:library/{}:pull", AUTH_URL, SVC_URL, image);
        let response = self.client
            .get(&url)
            .send()
            .await?;
        Ok(response.json::<AuthResponse>().await?)
    }

    pub async fn get_manifests_for_image(&self, token: &str, image: &str, digest: &str) -> Result<Manifests> {
        let url = format!("{}/v2/library/{}/manifests/{}", REGISTRY_URL, image, digest);
        let response = self.client
            .get(&url)
            .header(ACCEPT, "application/vnd.docker.distribution.manifest.list.v2+json")
            .header(ACCEPT, "application/vnd.docker.distribution.manifest.v1+json")
            .header(ACCEPT, "application/vnd.docker.distribution.manifest.v2+json")
            .bearer_auth(token)
            .send()
            .await?;
        Ok(response.json::<Manifests>().await?)
    }

    pub async fn get_oci_manifest(&self, token: &str, image: &str, digest: &str) -> Result<OCIManifest> {
        let url = format!("{}/v2/library/{}/manifests/{}", REGISTRY_URL, image, digest);
        let response = self.client
            .get(&url)
            .header(ACCEPT, "application/vnd.oci.image.manifest.v1+json")
            .bearer_auth(token)
            .send()
            .await?;
        Ok(response.json::<OCIManifest>().await?)
    }

    pub async fn get_blob(&self, token: &str, image: &str, digest: &str, dest: &Path) -> Result<()> {
        let url = format!("{}/v2/library/{}/blobs/{}", REGISTRY_URL, image, digest);
        let mut response = self.client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?;
        
        let mut file = TokioFile::create(dest).await?;

        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
        }

        Ok(())
    }

}

fn move_files(source_dir: &Path, target_dir: &Path) -> Result<()> {
    // Check if source directory exists and is a directory
    if !source_dir.is_dir() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Source directory not found").into());
    }

    // Create target directory if it doesn't exist
    if !target_dir.exists() {
        fs::create_dir_all(target_dir)?;
    }

    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let path = entry.path();
        println!("moving file {}" , path.display());
        
        // Only move files, not directories
        if path.is_file() {
            let file_name = match path.file_name() {
                Some(name) => name,
                None => continue,
            };
            let target_path = target_dir.join(file_name);

            // Define copy options
            let mut options = CopyOptions::new();
            options.overwrite = true; // Overwrite existing files in the target directory

            // Move the file
            move_file(&path, &target_path, &options)?;
        }
    }

    Ok(())
}

pub fn get_manifest_for_platform(manifests: &Manifests, platform: &Platform) -> Option<Manifest> {
    manifests.manifests.iter().find(|m| {
        m.platform == *platform 
    }).map(|x| x.clone())
}

/// When commands execute they usually don't throw an error when status code is invalid,
/// so this will convert that for simple control flow
pub fn output_error_if_failed(output: Output) -> Result<()> {
    if !output.status.success() {
        bail!(
            "STDOUT: {}; STDERR: {}",
            str::from_utf8(&output.stdout)?,
            str::from_utf8(&output.stderr)?
        )
    }
    Ok(())
}

pub fn unpack_tar_gz(tar_gz: &Path, dest: &Path) -> Result<()> {
    let tar = GzDecoder::new(File::open(tar_gz)?);
    let mut archive = Archive::new(tar);
    archive.unpack(dest)?;
    Ok(())
}

/// Downloads layers from the registry and unpacks them into the layers directory
pub async fn download_layers(client: &DockerClient, token: &String, image_name: &String, layers: &Vec<String>) -> Result<()> {
    let layers_path = get_layers_path()?;
    let temp_dir_downloads = TempDir::new()?;
    for digest in layers {
        let dest = temp_dir_downloads.path().join(format!("{}.tgz", &digest));
        let layer_directory = layers_path.join(digest);
        
        if !layer_directory.exists() {
            client.get_blob(&token, &image_name, &digest, dest.as_path()).await?;
        }
    }
    for layer in layers {
        let layer_archive_path = temp_dir_downloads.path().join(format!("{layer}.tgz"));
        let layer_directory = layers_path.join(layer);
        if !layer_directory.exists() {
            unpack_tar_gz(&layer_archive_path, layer_directory.as_path().as_std_path())?;
        }
    }
    Ok(())
}

// Mounts the image and 
pub async fn mount_and_combine_layers(layers: Vec<String>, layers_path: &Path) -> Result<()> {
    let temp_dir_unpack = TempDir::new()?;
    for layer in layers {
        let layer_archive_path = layers_path.join(format!("{layer}.tgz"));
        let tar_gz = File::open(layer_archive_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(&temp_dir_unpack.path())?;
    }
    Ok(())
}

pub fn create_disk_image(image_path: &Utf8PathBuf, blocks: u64) -> Result<()>{
    output_error_if_failed(
    Command::new("dd")
        .args(&[
            "if=/dev/zero",
            format!("of={}", image_path.as_str()).as_str(),
            "bs=4k",
            format!("count={blocks}").as_str()
        ])
        .output()?
    )?;
    Ok(())
}

/// Formats an image file as ext4
pub fn format_ext4_file(path: &Utf8PathBuf) -> Result<()> {
    output_error_if_failed(
        Command::new("mkfs.ext4")
            .args(&[path.as_str()])
            .output()?
    )?;
    Ok(())
}

pub fn mount_image(image_path: &Utf8PathBuf, mount_path: &Utf8PathBuf) -> Result<()> {
    output_error_if_failed(
        Command::new("mount")
            .args(&[image_path.as_str(), mount_path.as_str()])
            .output()?
    )?;
    Ok(())
}

/// Dumps a local image contents to a folder
pub fn save_local_image(image_name: String, target_path: &Path) -> Result<()> {
    // Spawn the `docker save` command
    let mut docker_save = Command::new("docker")
        .arg("save")
        .arg(image_name)
        .stdout(Stdio::piped())
        .spawn()?;

    // Spawn the `tar -x -C <target_directory>` command
    let mut tar_extract = Command::new("tar")
        .arg("-x")
        .arg("-C")
        .arg(target_path)
        .stdin(Stdio::piped())
        .spawn()?;

    // Get the stdout of `docker save` and the stdin of `tar`
    if let Some(docker_stdout) = &mut docker_save.stdout {
        if let Some(tar_stdin) = tar_extract.stdin.as_mut() {
            // Pipe the output of `docker save` directly into `tar`
            io::copy(docker_stdout, tar_stdin)?;
        }
    }

    // Wait for the `tar` command to complete
    tar_extract.wait()?;
    // Wait for the `docker save` command to complete
    docker_save.wait()?;

    Ok(())
}

pub fn layer_digest_to_cache_id(digest: &String) -> Result<String> {
    let layer_cache_id_path_str = format!("/var/lib/docker/image/overlay2/layerdb/sha256/{}/cache-id", digest);
    dbg!(layer_cache_id_path_str.as_str());
    let layer_cache_id_path = Path::new(layer_cache_id_path_str.as_str());
    let cache_id = fs::read_to_string(layer_cache_id_path)?;
    Ok(cache_id)
}
pub fn umount_image(mount_path: &Utf8PathBuf) -> Result<()> {
    output_error_if_failed(
        Command::new("umount")
            .args(&[mount_path.as_str()])
            .output()?
    )?;
    Ok(())
}

pub fn copy_layers_to_path(layers_path: &Path, layers: &Vec<String>, target_path: &Path) -> Result<()> {
    for layer in layers {
        let source_layer_path = layers_path.join(layer);
        let target_layer_path = target_path.join(layer);
        println!("moving {} to {}", source_layer_path.display(), target_layer_path.display());
        if let Err(_) = move_files(&source_layer_path, &target_layer_path) {
            println!("Failed to move layer {}", layer);
        }
    }
    Ok(())
}

/// Creates a drive image from layers
/// returns the size of the newly created image
pub fn create_drive_image(digest: &str, layers: &Vec<String>, layers_path: &Path) -> Result<u64>{
    // Create two temp dirs, one for the mount and one for the unpacking
    let temp_combined_dir = TempDir::new()?;
    let temp_mount_dir = TempDir::new()?;
    let temp_mount_path = Utf8PathBuf::from_path_buf(temp_mount_dir.path().to_path_buf()).map_err(|e|{anyhow!("Failed to convert temp mount dir to utf8")})?;
    let image_directory = get_images_path()?;
    let image_path = image_directory.join(format!("{digest}.img"));
    fs::create_dir_all(&image_directory)?;
    if image_path.exists() { fs::remove_file(&image_path)?;}

    // Copy layers to the temporary directory
    copy_layers_to_path(layers_path, &layers, temp_combined_dir.path())?;
    let files: Vec<String> = fs::read_dir(temp_combined_dir.path()).unwrap_or_panic_json().map(|l|{l.unwrap().path().to_str().unwrap().to_string()}).collect();
    let image_size = fs_extra::dir::get_size(&temp_combined_dir.path())?;
    if image_size == 0 {
        bail!("Image size must be greater than 0");
    }
    let blocks = image_size / 4096;

    // Create file and mount it first so we write into it directly
    create_disk_image(&image_path, blocks)?;
    format_ext4_file(&image_path)?;
    mount_image(&image_path, &temp_mount_path)?;
    println!("Moving files");
    move_files(temp_combined_dir.path(), temp_mount_dir.path())?;
    println!("Moved files");
    // Unmount the image now that we're done
    umount_image(&temp_mount_path)?;
    Ok(image_size)
}

pub trait UnwrapOrPanicJson<T> {
    fn unwrap_or_panic_json(self) -> T;
}

impl<T, E> UnwrapOrPanicJson<T> for Result<T, E>
where
    E: Display,
{
    fn unwrap_or_panic_json(self) -> T {
        match self {
            Ok(value) => value,
            Err(e) => {
                let error_message = json!({ "error": e.to_string() }).to_string();
                panic!("{}", error_message);
            }
        }
    }
}

pub fn get_app_state_path() -> Result<Utf8PathBuf> {
    Ok(BASE_PATH.read().map_err(|_|{anyhow::anyhow!("Failed to get state path")})?.as_path().join("state.json"))
}

pub fn get_layers_path() -> Result<Utf8PathBuf> {
    Ok(BASE_PATH.read().map_err(|_|{anyhow::anyhow!("Failed to get layers path")})?.as_path().join("layers"))
}

pub fn get_docker_layers_path() -> Utf8PathBuf {
    Utf8PathBuf::from("/var/lib/docker/overlay2")
}

pub fn get_images_path() -> Result<Utf8PathBuf> {
    Ok(BASE_PATH.read().map_err(|_|{anyhow::anyhow!("Failed to get images path")})?.as_path().join("images"))
}