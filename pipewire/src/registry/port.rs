use crate::Direction;
use crate::store::Store;
use pipewire::keys::{AUDIO_CHANNEL, NODE_ID, PORT_DIRECTION, PORT_ID, PORT_MONITOR, PORT_NAME};
use pipewire::registry::GlobalObject;
use pipewire::spa::utils::dict::DictRef;

pub fn handle_port(id: u32, global: &GlobalObject<&DictRef>, store: &mut Store) {
    if let Some(props) = global.props {
        let node_id = props.get(*NODE_ID);
        let pid = props.get(*PORT_ID);
        let name = props.get(*PORT_NAME);
        let channel = props.get(*AUDIO_CHANNEL);
        let direction = props.get(*PORT_DIRECTION);
        let is_monitor = props.get(*PORT_MONITOR);

        if node_id.is_none()
            || pid.is_none()
            || name.is_none()
            || channel.is_none()
            || direction.is_none()
        {
            return;
        }

        let name = name.unwrap();
        let channel = channel.unwrap();

        let direction = match direction.unwrap() {
            "in" => Direction::In,
            "out" => Direction::Out,
            _ => return,
        };

        let is_monitor = if let Some(monitor) = is_monitor {
            monitor.parse::<bool>().unwrap_or_default()
        } else {
            false
        };

        let port = RegistryPort::new(id, name, channel, is_monitor);

        if let Some(node_id) = node_id.and_then(|s| s.parse::<u32>().ok())
            && let Some(port_id) = pid.and_then(|s| s.parse::<u32>().ok())
        {
            if store.unmanaged_device_node_get(node_id).is_some() {
                store.unmanaged_node_port_add(node_id, direction, port);
                return;
            }
            if let Some(node) = store.unmanaged_client_node_get(node_id) {
                node.add_port(port_id, direction, port);
                store.unmanaged_client_node_check(node_id);
            }
        }
    }
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub(crate) struct RegistryPort {
    pub global_id: u32,
    pub name: String,
    pub channel: String,
    pub is_monitor: bool,
}

impl RegistryPort {
    pub fn new(id: u32, name: &str, channel: &str, is_monitor: bool) -> Self {
        let name = name.to_string();
        let channel = channel.to_string();

        Self {
            global_id: id,
            name,
            channel,
            is_monitor,
        }
    }
}
