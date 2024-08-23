use core::str;
use std::{path::Path, process::{Command, Output}};
use anyhow::{bail, Result};
use camino::Utf8PathBuf;

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

/// Coppies all files in the source recursively to the target
pub fn copy_recursive(source: &Path, target: &Path) -> Result<()> {
    if !source.exists() {
        bail!("Source directory {} not found", source.display());
    }
    if !source.is_dir() {
        bail!("Source {} is not a directory", source.display());
    }
    let source_files = format!("{}/*", source.display());
    let target = target.display().to_string();
    // sleep(Duration::from_secs(200));
    output_error_if_failed(
        Command::new("sh")
        .arg("-c")
        .arg(format!("rsync -a {} {}", source_files, target))
        .output()?
    )?;
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

pub fn umount_image(mount_path: &Utf8PathBuf) -> Result<()> {
    output_error_if_failed(
        Command::new("umount")
            .args(&[mount_path.as_str()])
            .output()?
    )?;
    Ok(())
}

/// Burns the bootloader at the provided path to the MBR of the provided image
pub fn burn_bootloader(image_path: &Utf8PathBuf, bootloader_path: &Utf8PathBuf) -> Result<()> {
    output_error_if_failed(
        Command::new("dd")
            .args(&[
                format!("if={}", bootloader_path.as_str()).as_str(),
                format!("of={}", image_path.as_str()).as_str()
            ])
            .output()?
    )?;
    Ok(())
}

