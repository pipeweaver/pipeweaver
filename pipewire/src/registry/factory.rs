use crate::store::Store;
use anyhow::anyhow;
use pipewire::keys::{
    FACTORY_NAME, FACTORY_TYPE_NAME, FACTORY_TYPE_VERSION, MODULE_ID, OBJECT_SERIAL,
};
use pipewire::registry::GlobalObject;
use pipewire::spa::utils::dict::DictRef;
use pipewire::types::ObjectType;

pub fn handle_factory(id: u32, global: &GlobalObject<&DictRef>, store: &mut Store) {
    if let Some(props) = global.props
        && let Ok(factory) = RegistryFactory::try_from(props)
    {
        store.factory_add(id, factory);
    }
}

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct RegistryFactory {
    pub(crate) object_serial: u32,
    pub(crate) module_id: u32,

    pub(crate) name: String,
    pub(crate) factory_type: ObjectType,
    pub(crate) version: u32,
}

impl TryFrom<&DictRef> for RegistryFactory {
    type Error = anyhow::Error;

    fn try_from(value: &DictRef) -> Result<Self, Self::Error> {
        let object_serial = value
            .get(*OBJECT_SERIAL)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("OBJECT_SERIAL"))?;
        let module_id = value
            .get(*MODULE_ID)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("MODULE_ID"))?;
        let name = value
            .get(*FACTORY_NAME)
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("FACTORY_NAME"))?;
        let factory_type = value
            .get(*FACTORY_TYPE_NAME)
            .ok_or_else(|| anyhow!("FACTORY_TYPE_NAME"))?;
        let version = value
            .get(*FACTORY_TYPE_VERSION)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("FACTORY_VERSION"))?;

        Ok(RegistryFactory {
            object_serial,
            module_id,
            name,
            factory_type: crate::registry::to_object_type(factory_type),
            version,
        })
    }
}
