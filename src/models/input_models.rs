use clap::Args;

#[derive(Debug, Clone)]
pub struct ImageArg {
    /// The name of the image
    pub name: String,
    /// The tag of the image(eg: latest)
    pub tag: String,
}

impl From<String> for ImageArg {
    fn from(name: String) -> Self {
        let parts = name.split(":").collect::<Vec<&str>>();
        if parts.len() == 2 {
            ImageArg {
                name: parts[0].to_string(),
                tag: parts[1].to_string()
            }
        } else {
            ImageArg {
                name,
                tag: String::from("latest")
            }
        }
    }
}

impl ToString for ImageArg {
    fn to_string(&self) -> String {
        format!("{}:{}", self.name, self.tag)
    }
}

#[derive(Debug, Args)]
pub struct ImageInfoArgs {

    /// The image to get info about
    pub image: ImageArg,
    
    /// The operating system the image is for
    #[clap(long, default_value_t = String::from("linux"))]
    pub os: String,
    /// The architecture the image is for
    #[clap(long, default_value_t = String::from("amd64"))]
    pub architecture: String
}

#[derive(Debug, Args)]
pub struct BuildImageArgs {
    /// The image to pull
    pub image: ImageArg,
    /// The operating system the image is for
    #[clap(long, default_value_t = String::from("linux"))]
    pub os: String,
    /// The architecture the image is for
    #[clap(long, default_value_t = String::from("amd64"))]
    pub architecture: String,
}

#[derive(Debug, Args)]
pub struct RemoveImageArgs {
    /// The image to remove
    pub image: ImageArg,
    /// Removes all layers that were associated with the image as long as other images don't reference them
    #[clap(long)]
    pub prune: bool,
    /// The operating system the image is for
    #[clap(long)]
    pub os: Option<String>,
    /// The platform the image is for
    #[clap(long)]
    pub architecture: Option<String>
}

#[derive(Debug, Args)]
pub struct ListImagesArgs {
    /// The operating system the image is for
    #[clap(long)]
    os: Option<String>,
    /// The platform the image is for
    #[clap(long)]
    platform: Option<String>
}