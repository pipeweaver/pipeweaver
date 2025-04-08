use crate::handler::pipewire::manager::PipewireManager;
use anyhow::Result;

pub(crate) trait LoadProfile {
    async fn load_profile(&mut self) -> Result<()>;
}

impl LoadProfile for PipewireManager {
    async fn load_profile(&mut self) -> Result<()> {
        todo!()
    }
}

trait LoadProfileLocal {
    async fn create_nodes(&mut self) -> Result<()>;
    async fn create_filters(&mut self) -> Result<()>;

    async fn apply_routing(&mut self) -> Result<()>;
}

impl LoadProfileLocal for PipewireManager {
    async fn create_nodes(&mut self) -> Result<()> {
        todo!()
    }

    async fn create_filters(&mut self) -> Result<()> {
        todo!()
    }

    async fn apply_routing(&mut self) -> Result<()> {
        todo!()
    }
}