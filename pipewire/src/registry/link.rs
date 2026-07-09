use crate::store::Store;
use anyhow::anyhow;
use pipewire::keys::{
    LINK_INPUT_NODE, LINK_INPUT_PORT, LINK_OUTPUT_NODE, LINK_OUTPUT_PORT, OBJECT_SERIAL,
};
use pipewire::registry::GlobalObject;
use pipewire::spa::utils::dict::DictRef;

pub fn handle_link(id: u32, global: &GlobalObject<&DictRef>, store: &mut Store) {
    if let Some(props) = global.props
        && let Ok(link) = RegistryLink::try_from(props)
    {
        store.unmanaged_link_add(id, link);
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct RegistryLink {
    pub(crate) object_serial: u32,

    pub input_node: u32,
    pub input_port: u32,
    pub output_node: u32,
    pub output_port: u32,
}

impl TryFrom<&DictRef> for RegistryLink {
    type Error = anyhow::Error;

    fn try_from(value: &DictRef) -> Result<Self, Self::Error> {
        let object_serial = value
            .get(*OBJECT_SERIAL)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("OBJECT_SERIAL"))?;
        let input_node = value
            .get(*LINK_INPUT_NODE)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("LINK_INPUT_NODE"))?;
        let input_port = value
            .get(*LINK_INPUT_PORT)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("LINK_INPUT_PORT"))?;
        let output_node = value
            .get(*LINK_OUTPUT_NODE)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("LINK_OUTPUT_NODE"))?;
        let output_port = value
            .get(*LINK_OUTPUT_PORT)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("LINK_OUTPUT_PORT"))?;

        Ok(RegistryLink {
            object_serial,
            input_node,
            input_port,
            output_node,
            output_port,
        })
    }
}
