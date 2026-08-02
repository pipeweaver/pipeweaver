use crate::client::Client;
use anyhow::{Result, anyhow};

use crate::commands::{DaemonRequest, DaemonResponse, DaemonStatus};

// reqwest's blocking and async clients are unrelated types (not just an async/await
// difference), so this is the one spot maybe_async can't help with directly. Alias
// whichever one matches our feature under a single name and the rest of the body
// below is identical for both builds.
#[cfg(not(feature = "sync"))]
use reqwest::Client as HttpClient;
#[cfg(feature = "sync")]
use reqwest::blocking::Client as HttpClient;

#[derive(Debug)]
#[allow(unused)]
pub struct WebClient {
    url: String,
    status: DaemonStatus,
}

impl WebClient {
    pub fn connect(url: String) -> Result<Self> {
        Ok(Self::new(url))
    }

    pub fn new(url: String) -> Self {
        Self {
            url,
            status: DaemonStatus::default(),
        }
    }
}

#[maybe_async::maybe_async(?Send)]
impl Client for WebClient {
    async fn send(&mut self, request: &DaemonRequest) -> Result<DaemonResponse> {
        HttpClient::new()
            .post(&self.url)
            .json(&request)
            .send()
            .await?
            .json::<DaemonResponse>()
            .await
            .map_err(|e| e.into())
    }

    async fn get_status(&mut self) -> Result<DaemonStatus> {
        let status = self.send(&DaemonRequest::GetStatus).await?;
        match status {
            DaemonResponse::Status(status) => Ok(status),
            DaemonResponse::Err(error) => Err(anyhow!("{}", error)),
            _ => Err(anyhow!("Expected Status response, got {:?}", status)),
        }
    }
}
