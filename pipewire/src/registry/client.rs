use crate::store::Store;
use anyhow::anyhow;
use log::debug;
use pipewire::client::{Client, ClientChangeMask, ClientListener};
use pipewire::keys::{
    ACCESS, APP_NAME, APP_PROCESS_BINARY, MODULE_ID, OBJECT_SERIAL, PROTOCOL, SEC_GID, SEC_PID,
    SEC_UID,
};
use pipewire::registry::{GlobalObject, Registry};
use pipewire::spa::utils::dict::DictRef;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub fn handle_client(
    id: u32,
    global: &GlobalObject<&DictRef>,
    registry: Rc<RefCell<Registry>>,
    store: &mut Store,
    listener_store: Weak<RefCell<Store>>,
) {
    if let Some(props) = global.props {
        if let Ok(mut client) = RegistryClient::try_from(props) {
            let proxy: Option<Client> = registry.borrow().bind(global).ok();
            if let Some(client_proxy) = proxy {
                let info_local = listener_store.clone();
                let listener = client_proxy
                    .add_listener_local()
                    .info(move |info| {
                        for change in info.change_mask().iter() {
                            if change == ClientChangeMask::PROPS
                                && let Some(props) = info.props()
                                && let Some(process) = props.get(*APP_PROCESS_BINARY)
                                && let Some(info_local) = info_local.upgrade()
                            {
                                info_local
                                    .borrow_mut()
                                    .unmanaged_client_set_binary(id, String::from(process));
                            }
                        }
                    })
                    .register();
                client._listener = Some(listener);
                client._proxy = Some(client_proxy);

                store.unmanaged_client_add(id, client);
            }
        } else {
            debug!("Failed to create client: {:?}", props);
        }
    }
}

#[allow(unused)]
pub(crate) struct RegistryClient {
    object_serial: u32,

    module_id: u32,
    protocol: String,
    process_id: u32,
    user_id: u32,
    group_id: u32,
    access: String,
    pub(crate) application_name: String,
    pub(crate) application_binary: Option<String>,

    pub(crate) _proxy: Option<Client>,
    pub(crate) _listener: Option<ClientListener>,

    pub(crate) nodes: Vec<u32>,
}

impl RegistryClient {
    pub fn add_node(&mut self, id: u32) {
        self.nodes.push(id);
    }
}

impl TryFrom<&DictRef> for RegistryClient {
    type Error = anyhow::Error;

    fn try_from(value: &DictRef) -> Result<Self, Self::Error> {
        // I currently expect all these fields to be present for general usage
        let object_serial = value
            .get(*OBJECT_SERIAL)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("OBJECT_SERIAL"))?;
        let module_id = value
            .get(*MODULE_ID)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("MODULE_ID"))?;
        let protocol = value
            .get(*PROTOCOL)
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("PROTOCOL"))?;
        let process_id = value
            .get(*SEC_PID)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("SEC_PID"))?;
        let user_id = value
            .get(*SEC_UID)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("SEC_UID"))?;
        let group_id = value
            .get(*SEC_GID)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("SEC_GID"))?;
        let access = value
            .get(*ACCESS)
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("ACCESS"))?;
        let application_name = value
            .get(*APP_NAME)
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("APP_NAME"))?;

        Ok(Self {
            object_serial,
            module_id,
            protocol,
            process_id,
            user_id,
            group_id,
            access,

            application_name,
            application_binary: None,

            _proxy: None,
            _listener: None,

            nodes: vec![],
        })
    }
}
