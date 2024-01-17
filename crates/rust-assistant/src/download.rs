use crate::CrateVersion;
use reqwest::{Client, ClientBuilder};
use std::io::Read;

#[derive(Debug, Default, Clone)]
pub struct CrateDownloader {
    client: Client,
}

impl From<Client> for CrateDownloader {
    fn from(client: Client) -> Self {
        Self { client }
    }
}

impl TryFrom<ClientBuilder> for CrateDownloader {
    type Error = reqwest::Error;

    fn try_from(value: ClientBuilder) -> Result<Self, Self::Error> {
        Ok(Self {
            client: value.build()?,
        })
    }
}

impl CrateDownloader {
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
