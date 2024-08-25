use core::str;
use std::{fmt::Display, fs::{self, File}, io, path::Path, process::{Command, Output, Stdio}, thread::sleep, time::Duration};
use anyhow::{anyhow, bail, Context, Result};
use camino::Utf8PathBuf;
use flate2::read::GzDecoder;
use serde_json::json;
use tar::Archive;
use tempfile::TempDir;


use crate::{cli_commands::{burn_bootloader, copy_recursive, create_disk_image, create_loop_device, create_partition_table, detach_loop_device, format_ext4_file, mount_file, mount_with_offset, unmount_file}, docker_client::DockerClient, models::registry_models::{Manifest, Platform}, paths::get_images_path};


pub fn unpack_tar_gz(tar_gz: &Path, dest: &Path) -> Result<()> {
    let tar = GzDecoder::new(File::open(tar_gz)?);
    let mut archive = Archive::new(tar);
    archive.unpack(dest)?;
    Ok(())
}

/// Decompresses the layers and stores them in the output path
pub fn decompress_layers(layers: &Vec<String>, layers_path: &Path, output_path: &Path) -> Result<()> {
    for layer in layers {
        let layer_archive_path = layers_path.join(format!("{layer}.tgz"));
        if !layer_archive_path.exists() {
            bail!("Layer archive {} not found", layer_archive_path.display());
        }
        let tar_gz = File::open(layer_archive_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(output_path)?;
    }
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

/// Creates a drive image from layers
/// returns the size of the newly created image
pub fn create_drive_image(layers: &Vec<String>, layers_path: &Path, bootloader_path: &String, image_path: &Utf8PathBuf) -> Result<u64>{
    // Create two temp dirs, one for the mount and one for the unpacking
    let temp_combined_dir = TempDir::new()?;
    let temp_bootloader_dir = TempDir::new()?;
    let temp_mount_dir = TempDir::new()?;
    
    if image_path.exists() { fs::remove_file(&image_path)?;}
    // Copy layers to the temporary directory
    decompress_layers(&layers, layers_path, temp_combined_dir.path())?;
    let mut image_size = fs_extra::dir::get_size(&temp_combined_dir.path())? * 2;
    
    if image_size == 0 {
        bail!("Image size must be greater than 0");
    }
    image_size += 1024 * 1024 * 20; // Add 20MB to the image size for the partition table and bootloader
    let blocks = image_size / 4096;

    // Create file and mount it so we can copy the files into it
    create_disk_image(&image_path, blocks)?;
    create_partition_table(&image_path)?;
    let loop_device = create_loop_device()?;
    // Mount the loop device to the image with a 1MB offset
    mount_with_offset(&image_path, &loop_device, 1024 * 1024)?;
    format_ext4_file(&loop_device.to_string())?;
    // Mount the loop device to the temp mount directory
    mount_file(&loop_device.to_string(), &temp_mount_dir.path().to_str().context("Failed to convert temp mount dir to string")?.to_string())?;
    println!("Mounted loop device to temp mount dir at {}", temp_mount_dir.path().display());
    // sleep(Duration::from_secs(300));
    copy_recursive(temp_combined_dir.path(), temp_mount_dir.path())?;
    println!("Copied files to temp mount dir");
    // Copy the bootloader locally so we can use it after unmounting the image
    let target_bootloader_path = Utf8PathBuf::from_path_buf(temp_bootloader_dir.path().join("bootloader.img")).map_err(|e|{anyhow!("Failed to convert temp mount dir to utf8")})?;
    let bootloader_source_path = temp_mount_dir.path().join(bootloader_path.strip_prefix("/").context("Failed to strip prefix")?);
    fs::copy(bootloader_source_path.as_path(), target_bootloader_path.as_path())?;
    println!("Waiting to allow inspection of loop device {} and bootloader {}", loop_device.to_string(), target_bootloader_path.to_string());
    // sleep(Duration::from_secs(300));
    // Unmount the image now that we're done
    unmount_file(&loop_device)?;
    // Detach the loop device
    detach_loop_device(&loop_device.to_string())?;
    // And finally, burn the bootloader
    burn_bootloader(&image_path, &target_bootloader_path)?;
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

