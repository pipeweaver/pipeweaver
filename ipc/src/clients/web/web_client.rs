use crate::client::Client;
use anyhow::{Result, anyhow};

use crate::commands::{DaemonRequest, DaemonResponse, DaemonStatus};
use anyhow::bail;
use async_trait::async_trait;

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

#[async_trait]
impl Client for WebClient {
    async fn send(&mut self, request: DaemonRequest) -> Result<DaemonResponse> {
        reqwest::Client::new()
            .post(&self.url)
            .json(&request)
            .send()
            .await?
            .json::<DaemonResponse>()
            .await
            .map_err(|e| e.into())
    }

    async fn get_status(&mut self) -> Result<DaemonStatus> {
        let status = self.send(DaemonRequest::GetStatus).await?;
        match status {
            DaemonResponse::Status(status) => Ok(status),
            DaemonResponse::Err(error) => Err(anyhow!("{}", error)),
            _ => Err(anyhow!("Expected Status response, got {:?}", status)),
        }
    }
}
