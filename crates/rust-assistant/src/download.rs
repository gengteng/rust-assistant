use crate::CrateVersion;
use bytes::Bytes;
use reqwest::{Client, ClientBuilder};

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
    pub async fn download_crate_file(&self, crate_version: &CrateVersion) -> anyhow::Result<Bytes> {
        let url = format!(
            "https://static.crates.io/crates/{}/{}-{}.crate",
            crate_version.krate, crate_version.krate, crate_version.version
        );

        let resp = self.client.get(url).send().await?;

        if !resp.status().is_success() {
            anyhow::bail!("Http status is not 200: {}", resp.text().await?);
        }

        Ok(resp.bytes().await?)
    }
}
