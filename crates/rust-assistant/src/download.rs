//! The `download` module.
//!
//! Responsible for downloading crates and their contents from sources like crates.io.
//! This module likely includes structures like `CrateDownloader` which handle the intricacies
//! of making network requests, handling responses, and processing the downloaded data.
//!
use crate::CrateVersion;
use reqwest::{Client, ClientBuilder};
use std::io::Read;

/// The `CrateDownloader` struct, responsible for downloading crate files from the internet.
///
/// This struct uses the `reqwest` crate's `Client` to make HTTP requests for crate files.
#[derive(Debug, Default, Clone)]
pub struct CrateDownloader {
    client: Client,
}

impl From<Client> for CrateDownloader {
    /// Creates a `CrateDownloader` from a `reqwest::Client`.
    ///
    /// This allows for custom configuration of the HTTP client used for downloading.
    ///
    fn from(client: Client) -> Self {
        Self { client }
    }
}

impl TryFrom<ClientBuilder> for CrateDownloader {
    type Error = reqwest::Error;

    /// Tries to create a `CrateDownloader` from a `reqwest::ClientBuilder`.
    ///
    /// This method attempts to build a `reqwest::Client` and returns a `CrateDownloader` if successful.
    ///
    fn try_from(value: ClientBuilder) -> Result<Self, Self::Error> {
        Ok(Self {
            client: value.build()?,
        })
    }
}

impl CrateDownloader {
    /// Asynchronously downloads a crate file from crates.io.
    ///
    /// This method constructs the URL for the crate file based on the provided `CrateVersion`
    /// and uses the internal HTTP client to download it.
    ///
    pub async fn download_crate_file(
        &self,
        crate_version: &CrateVersion,
    ) -> anyhow::Result<Vec<u8>> {
        let url = format!(
            "https://static.crates.io/crates/{}/{}-{}.crate",
            crate_version.krate, crate_version.krate, crate_version.version
        );

        let resp = self.client.get(url).send().await?;

        if !resp.status().is_success() {
            anyhow::bail!("Http status is not 200: {}", resp.text().await?);
        }

        let compressed_data = resp.bytes().await?;

        let data = tokio::task::spawn_blocking(move || {
            let mut dc = flate2::bufread::GzDecoder::new(compressed_data.as_ref());
            let mut tar_data = Vec::new();
            dc.read_to_end(&mut tar_data)?;

            Ok::<_, anyhow::Error>(tar_data)
        })
        .await??;

        Ok(data)
    }
}
