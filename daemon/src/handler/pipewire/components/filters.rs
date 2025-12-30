use crate::handler::pipewire::components::audio_filters::meter::MeterFilter;
use crate::handler::pipewire::components::audio_filters::pass_through::PassThroughFilter;
use crate::handler::pipewire::components::audio_filters::volume::VolumeFilter;
use crate::handler::pipewire::manager::PipewireManager;
use crate::{APP_ID, APP_NAME, APP_NAME_ID};
use anyhow::{Result, bail};
use pipeweaver_pipewire::oneshot;
use pipeweaver_pipewire::{FilterProperties, FilterValue, MediaClass, PipewireMessage};
use ulid::Ulid;

#[allow(unused)]
pub(crate) trait FilterManagement {
    async fn filter_pass_create(&mut self, name: String) -> Result<Ulid>;
    async fn filter_pass_create_id(&mut self, name: String, id: Ulid) -> Result<()>;

    async fn filter_volume_create(&mut self, name: String) -> Result<Ulid>;
    async fn filter_volume_create_id(&mut self, name: String, id: Ulid) -> Result<()>;

    async fn filter_meter_create(&mut self, node: Ulid, name: String) -> Result<Ulid>;
    async fn filter_meter_create_id(&mut self, node: Ulid, name: String, id: Ulid) -> Result<()>;
    async fn filter_meter_load(&mut self, meter: Ulid) -> Result<()>;

    async fn filter_volume_set(&self, id: Ulid, volume: u8) -> Result<()>;

    async fn filter_remove(&mut self, id: Ulid) -> Result<()>;
}

impl FilterManagement for PipewireManager {
    async fn filter_pass_create(&mut self, name: String) -> Result<Ulid> {
        let id = Ulid::new();
        self.filter_pass_create_id(name, id).await?;

        Ok(id)
    }
    async fn filter_pass_create_id(&mut self, name: String, id: Ulid) -> Result<()> {
        let props = self.filter_pass_get_props(name, id);
        self.filter_pw_create(props).await
    }

    async fn filter_volume_create(&mut self, name: String) -> Result<Ulid> {
        let id = Ulid::new();
        self.filter_volume_create_id(name, id).await?;

        Ok(id)
    }
    async fn filter_volume_create_id(&mut self, name: String, id: Ulid) -> Result<()> {
        let props = self.filter_volume_get_props(name, id);
        self.filter_pw_create(props).await
    }

    async fn filter_meter_create(&mut self, node: Ulid, name: String) -> Result<Ulid> {
        let id = Ulid::new();
        self.filter_meter_create_id(node, name, id).await?;

        Ok(id)
    }

    async fn filter_meter_create_id(&mut self, node: Ulid, name: String, id: Ulid) -> Result<()> {
        let props = self.filter_meter_get_props(node, name, id);
        self.filter_pw_create(props).await
    }

    async fn filter_meter_load(&mut self, meter: Ulid) -> Result<()> {
        let enabled = self.meter_enabled;
        let message = PipewireMessage::SetFilterValue(meter, 0, FilterValue::Bool(enabled));
        self.pipewire().send_message(message)?;

        Ok(())
    }

    async fn filter_volume_set(&self, id: Ulid, volume: u8) -> Result<()> {
        if !(0..=100).contains(&volume) {
            bail!("Volume must be between 0 and 100");
        }

        let value = FilterValue::UInt8(volume);
        let message = PipewireMessage::SetFilterValue(id, 0, value);
        let _ = self.pipewire().send_message(message);

        Ok(())
    }

    async fn filter_remove(&mut self, id: Ulid) -> Result<()> {
        self.filter_pw_remove(id).await
    }
}

trait FilterManagementLocal {
    async fn filter_pw_create(&self, props: FilterProperties) -> Result<()>;
    async fn filter_pw_remove(&self, id: Ulid) -> Result<()>;

    fn filter_pass_get_props(&self, name: String, id: Ulid) -> FilterProperties;
    fn filter_volume_get_props(&self, name: String, id: Ulid) -> FilterProperties;
    fn filter_meter_get_props(&self, node: Ulid, name: String, id: Ulid) -> FilterProperties;
}

impl FilterManagementLocal for PipewireManager {
    async fn filter_pw_create(&self, mut props: FilterProperties) -> Result<()> {
        let (send, recv) = oneshot::channel();

        props.ready_sender = Some(send);
        self.pipewire()
            .send_message(PipewireMessage::CreateFilterNode(props))?;
        recv.await?;

        Ok(())
    }

    async fn filter_pw_remove(&self, id: Ulid) -> Result<()> {
        let message = PipewireMessage::RemoveFilterNode(id);
        self.pipewire().send_message(message)
    }

    fn filter_pass_get_props(&self, name: String, id: Ulid) -> FilterProperties {
        let description = name.to_lowercase().replace(" ", "-");

        FilterProperties {
            filter_id: id,
            filter_name: "Pass-Through".into(),
            filter_nick: name.to_string(),
            filter_description: format!("{}/{}", APP_NAME_ID, description),

            class: MediaClass::Duplex,
            app_id: APP_ID.to_string(),
            app_name: APP_NAME.to_string(),
            linger: false,
            callback: Box::new(PassThroughFilter::new()),

            ready_sender: None,
        }
    }

    fn filter_volume_get_props(&self, name: String, id: Ulid) -> FilterProperties {
        let description = name.to_lowercase().replace(" ", "-");

        FilterProperties {
            filter_id: id,
            filter_name: "Volume".into(),
            filter_nick: name.to_string(),
            filter_description: format!("{}/{}", APP_NAME_ID, description),

            class: MediaClass::Duplex,
            app_id: APP_ID.to_string(),
            app_name: APP_NAME.to_string(),
            linger: false,
            callback: Box::new(VolumeFilter::new(0)),

            ready_sender: None,
        }
    }

    fn filter_meter_get_props(&self, node: Ulid, name: String, id: Ulid) -> FilterProperties {
        let description = name.to_lowercase().replace(" ", "-");

        FilterProperties {
            filter_id: id,
            filter_name: "Meter".into(),
            filter_nick: name.to_string(),
            filter_description: format!("{}/{}", APP_NAME_ID, description),

            class: MediaClass::Source,
            app_id: APP_ID.to_string(),
            app_name: APP_NAME.to_string(),
            linger: false,
            callback: Box::new(MeterFilter::new(node, self.meter_callback.clone())),

            ready_sender: None,
        }
    }
}
