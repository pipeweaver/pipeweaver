use crate::client::Client;
use anyhow::Result;

use crate::commands::{
    APICommand, APICommandResponse, DaemonRequest, DaemonResponse, DaemonStatus,
};
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

    fn new(url: String) -> Self {
        Self {
            url,
            status: DaemonStatus::default(),
        }
    }
}

#[async_trait]
impl Client for WebClient {
    async fn send(&mut self, request: DaemonRequest) -> anyhow::Result<()> {
        let resp = reqwest::Client::new()
            .post(&self.url)
            .json(&request)
            .send()
            .await?
            .json::<DaemonResponse>()
            .await?;

        // Should probably abstract this part, it's common between clients..
        match resp {
            DaemonResponse::Status(status) => {
                self.status = status.clone();
                Ok(())
            }
            DaemonResponse::Ok => Ok(()),
            DaemonResponse::Err(error) => bail!("{}", error),
            DaemonResponse::Patch(_) => bail!("Received PATCH!"),
            DaemonResponse::Pipewire(response) => match response {
                APICommandResponse::Id(_) => Ok(()),
                APICommandResponse::Ok => Ok(()),
                APICommandResponse::Err(error) => bail!("{}", error),
            },
        }
    }

    async fn poll_status(&mut self) -> anyhow::Result<()> {
        self.send(DaemonRequest::GetStatus).await
    }

    async fn command(&mut self, command: APICommand) -> anyhow::Result<()> {
        let command = DaemonRequest::Pipewire(command);
        self.send(command).await
    }

    fn status(&self) -> &DaemonStatus {
        &self.status
    }
}
