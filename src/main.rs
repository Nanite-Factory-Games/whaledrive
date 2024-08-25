use std::fs;

use camino::Utf8PathBuf;

use serde_json::json;
use whaledrive::{cli_commands::check_required_commands_exist, models::input_models::{BuildImageArgs, ImageInfoArgs, RemoveImageArgs}, paths::BASE_PATH, utils::UnwrapOrPanicJson};
use anyhow::Result;
use tokio;
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(name = "whaledrive", version)]
pub struct App {
    #[clap(flatten)]
    pub global_opts: GlobalOpts,

    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Get info about an image
    Info(ImageInfoArgs),
    /// Create image from a registry
    Build(BuildImageArgs),
    /// List all images that are currently stored
    Images,
    /// Remove an image
    Rm(RemoveImageArgs),
    /// Remove all images not refered to by a tag and all layers nor associated with an image
    Prune,
}

#[derive(Debug, Args)]
pub struct GlobalOpts {
    /// Folder where this utility will store things
    #[clap(long, short, global = true, default_value_t = Utf8PathBuf::try_from(std::env::current_dir().unwrap_or_panic_json().join("data")).unwrap_or_panic_json())]
    base_path: Utf8PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    // We wrap most of the main logic in a function so we can
    // catch any errors and print them nicely
    match wrapped_main().await {
        Ok(result) => println!("{result}"),
        Err(e) => eprintln!("{}", json!({
            "error": e.to_string()
        }).to_string())
    }
    Ok(())
}


async fn wrapped_main() -> Result<String> {
    // Check if all required commands exist
    check_required_commands_exist()?;

    let command = App::parse();

    // Update global base path based on CLI arguments
    let mut base_path_lock = BASE_PATH.write().map_err(|_|{anyhow::anyhow!("Failed to get state path lock")})?;
    *base_path_lock = command.global_opts.base_path.clone();
    // Create the base path if it doesn't exist
    fs::create_dir_all(command.global_opts.base_path.as_path())?;

    // Needs to be dropped early so we don't get deadlocked
    drop(base_path_lock);

    match command.command {
        Command::Info(args) => whaledrive::commands::image_info(args).await,
        Command::Build(args) => whaledrive::commands::build_image(args).await,
        Command::Prune => whaledrive::commands::prune(),
        Command::Images => whaledrive::commands::list_images(),
        Command::Rm(args) => whaledrive::commands::remove_image(args),
    }
}