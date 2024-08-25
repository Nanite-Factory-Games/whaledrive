use core::str;
use std::{io::Write, path::Path, process::{Command, Output, Stdio}};
use anyhow::{bail, Result};
use camino::Utf8PathBuf;
use clap::builder::Str;

pub fn check_required_commands_exist() -> Result<()> {
    which::which("dd")?;
    which::which("losetup")?;
    which::which("mkfs.ext4")?;
    which::which("mount")?;
    which::which("sfdisk")?;
    Ok(())
}

/// When commands execute they usually don't throw an error when status code is invalid,
/// so this will convert that for simple control flow
pub fn output_error_if_failed(output: Output) -> Result<String> {
    if !output.status.success() {
        bail!(
            "STDOUT: {}; STDERR: {}",
            str::from_utf8(&output.stdout)?,
            str::from_utf8(&output.stderr)?
        )
    }
    Ok(String::from_utf8(output.stdout)?)
}

/// Coppies all files in the source recursively to the target
pub fn copy_recursive(source: &Path, target: &Path) -> Result<()> {
    let source_files = format!("{}/*", source.display());
    let target = target.display().to_string();
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

/// Formats a image or device file as ext4
pub fn format_ext4_file(path: &String) -> Result<()> {
    println!("Formatting {} to ext4", path);
    output_error_if_failed(
        Command::new("mkfs.ext4")
            .args(&[path.as_str()])
            .output()?
    )?;
    Ok(())
}

/// Mounts an image or device file to a mount path
pub fn mount_file(image_path: &String, mount_path: &String) -> Result<()> {
    output_error_if_failed(
        Command::new("mount")
            .args(&[
                "-t", "auto",
                image_path.as_str(),
                mount_path.as_str()
            ])
            .output()?
    )?;
    Ok(())
}

pub fn unmount_file(mount_path: &Utf8PathBuf) -> Result<()> {
    output_error_if_failed(
        Command::new("umount")
            .args(&[mount_path.as_str()])
            .output()?
    )?;
    Ok(())
}

/// Burns the bootloader at the provided path to the MBR of the provided image
pub fn burn_bootloader(image_path: &Utf8PathBuf, bootloader_path: &Utf8PathBuf) -> Result<()> {
    // A block size of 440 and the no truncation flag is required to make sure
    // That only the number of bytes that are allowed to be written to the mbr are written
    output_error_if_failed(
        Command::new("dd")
            .args(&[
                format!("if={}", bootloader_path.as_str()).as_str(),
                format!("of={}", image_path.as_str()).as_str(),
                "bs=440",
                "count=1",
                "conv=notrunc"
            ])
            .output()?
    )?;
    Ok(())
}

/// Uses sfdisk to create a partition table on the provided image
/// with a single ext4 partition
pub fn create_partition_table(image_path: &Utf8PathBuf) -> Result<()> {

    let mut sfdisk = Command::new("sfdisk")
        .arg(&image_path)
        .stdin(Stdio::piped())  // Enable piping to stdin
        .stdout(Stdio::piped())
        .spawn()?;

    // Write the partition data to sfdisk's stdin
    if let Some(mut stdin) = sfdisk.stdin.take() {
        // Type 83 is ext4. We also mark the partition as bootable
        stdin.write_all(b"type=83,bootable\n")?;
    }

    output_error_if_failed(sfdisk.wait_with_output()?)?;

    Ok(())
}

/// Uses losetup to create a loop device, returning its path
pub fn create_loop_device()-> Result<Utf8PathBuf> {
    let path = output_error_if_failed(
        Command::new("losetup")
            .args(&[
                "-f",
            ])
            .output()?
    )?;
    Ok(Utf8PathBuf::from(path.trim()))
}

/// Uses losetup to detach a specific loop device
pub fn detach_loop_device(device_name: &String)-> Result<()> {
    output_error_if_failed(
        Command::new("losetup")
            .args(&[
                "-d",
                device_name.as_str()
            ])
            .output()?
    )?;
    Ok(())
}

/// Uses losetup to attach a specific loop device with a provided offset in bytes
pub fn mount_with_offset(image_path: &Utf8PathBuf, mount_path: &Utf8PathBuf, offset: u64) -> Result<()> {
    output_error_if_failed(
        Command::new("losetup")
            .args(&[
                "-o", &format!("{offset}"),
                mount_path.as_str(),
                image_path.as_str()
            ])
            .output()?
    )?;
    Ok(())
}