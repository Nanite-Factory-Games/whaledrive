use std::{fs, path::Path};

use hyper::header::ACCEPT;
use reqwest::Client;
use tokio::io::AsyncWriteExt;

use crate::{models::registry_models::{AuthResponse, ImageConfig, Manifests, OCIManifest}, paths::get_layers_compressed_path};
use anyhow::{bail, Context, Result};
use tokio::fs::File as TokioFile;


const REGISTRY_URL: &str = "https://registry-1.docker.io";
const AUTH_URL: &str = "https://auth.docker.io";
const SVC_URL: &str = "registry.docker.io";

pub struct DockerClient {
    client: Client,
    image_name: String,
    image_tag: String,
    namespace: String,
    token: String
}

impl DockerClient {

    pub async fn new_with_auth(image: &String) -> Result<DockerClient> {
        let mut namespace = String::from("library");
        let mut image_name = image.clone();
        let mut image_tag = String::from("latest");
        let client = reqwest::Client::new();
        if image.contains("/") {
            let parts = image.split("/").collect::<Vec<&str>>();
            namespace = parts[0].to_string();
            image_name = parts[1].to_string();
        }
        let image_name_parts = image_name.split(":").map(String::from).collect::<Vec<String>>();
        image_name = image_name_parts[0].to_string();
        if image_name_parts.len() > 1 {
            image_tag = image_name_parts[1].to_string();
        }
        let url = format!("{AUTH_URL}/token?service={SVC_URL}&scope=repository:{namespace}/{image_name}:pull");
        let response = client.get(&url).send().await?;
        let auth_response = response.json::<AuthResponse>().await?;
        Ok(
            Self {
                client,
                image_name,
                image_tag,
                namespace,
                token: auth_response.token
            }
        )
    }

    pub async fn get_manifests(&self) -> Result<Manifests> {

        let url = format!("{REGISTRY_URL}/v2/{}/{}/manifests/{}", self.namespace, self.image_name, self.image_tag);
        println!("Getting manifests for {}", url);
        let response = self.client
            .get(&url)
            // .header(ACCEPT, "application/vnd.docker.distribution.manifest.list.v2+json")
            // .header(ACCEPT, "application/vnd.docker.distribution.manifest.v1+json")
            .header(ACCEPT, "application/vnd.docker.distribution.manifest.v2+json")
            .bearer_auth(self.token.clone())
            .send()
            .await?;
        let manifest = response.json::<Manifests>().await?;
        if let Some(errors) = manifest.errors {
            bail!("Error getting manifests: {:?}", errors.get(0).context("Errors present but empty")?.message);
        }
        Ok(manifest)
    }

    pub async fn get_oci_manifest(&self,digest: &str) -> Result<OCIManifest> {
        let url = format!("{REGISTRY_URL}/v2/{}/{}/manifests/{}", self.namespace, self.image_name, digest);

        let response = self.client
            .get(&url)
            .header(ACCEPT, "application/vnd.oci.image.manifest.v1+json")
            .bearer_auth(self.token.clone())
            .send()
            .await?;
        Ok(response.json::<OCIManifest>().await?)
    }

    pub async fn download_layer(&self, digest: &str, dest: &Path) -> Result<()> {
        let url = format!("{REGISTRY_URL}/v2/{}/{}/blobs/{}", self.namespace, self.image_name, digest);
        let mut response = self.client
            .get(&url)
            .bearer_auth(self.token.clone())
            .send()
            .await?;
        
        let mut file = TokioFile::create(dest).await?;

        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
        }

        Ok(())
    }

    pub async fn get_image_config(&self, digest: &str) -> Result<ImageConfig> {
        let url = format!("{REGISTRY_URL}/v2/{}/{}/blobs/{}", self.namespace, self.image_name, digest);
        let response = self.client
            .get(&url)
            .bearer_auth(self.token.clone())
            .send()
            .await?;

        Ok(response.json::<ImageConfig>().await?)
    }

    /// Downloads layers from the registry and leaves them compressed
    pub async fn download_layers_compressed(&self, layers: &Vec<String>) -> Result<()> {
        let compressed_layers_path = get_layers_compressed_path()?;
        fs::create_dir_all(&compressed_layers_path)?;
        for digest in layers {
            let dest = compressed_layers_path.join(format!("{}.tgz", &digest));
            
            if !dest.exists() {
                self.download_layer(&digest, dest.as_path().as_std_path()).await?;
            }
        }
        Ok(())
    }

}