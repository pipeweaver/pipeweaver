//! Tracks which node is the configured/actual default sink and source, and
//! tells upstream when that changes. Also owns the small amount of
//! node-lookup-by-name/target logic that only exists to serve that purpose.

use super::Store;
use crate::default_device::{DefaultDefinition, DefaultDevice};
use crate::{MediaClass, NodeTarget, PipewireReceiver};
use anyhow::{Result, anyhow, bail};
use log::debug;
use pipewire::keys::MEDIA_CLASS;

impl Store {
    // ----- SET DEFAULT DEVICES -----
    pub fn set_default_sink(&mut self, device: DefaultDefinition) {
        let changed = self.default_sink.set(device);

        if changed && self.find_default_sink_id() {
            debug!("Default Sink Updated to: {:?}", self.default_sink);
            self.send_default_sink();
        }
    }

    pub fn set_default_source(&mut self, device: DefaultDefinition) {
        let changed = self.default_source.set(device);

        if changed && self.find_default_source_id() {
            debug!("Default Source Updated to: {:?}", self.default_source);
            self.send_default_source();
        }
    }

    /// Visible to the rest of `store` (see `nodes` / `unmanaged_devices`)
    /// because a node appearing, disappearing, or getting its pipewire id
    /// assigned can all change who the default sink is.
    pub(super) fn send_default_sink(&self) {
        self.send_default_update(&self.default_sink, MediaClass::Sink);
    }

    pub(super) fn send_default_source(&self) {
        self.send_default_update(&self.default_source, MediaClass::Source);
    }

    fn send_default_update(&self, default: &DefaultDevice, class: MediaClass) {
        if let Some(node_id) = default.get_active_node_id() {
            let message = if self.is_managed_node(node_id) {
                let ulid = self.managed_node_find_by_node_id(node_id).unwrap();
                PipewireReceiver::DefaultChanged(class, NodeTarget::Node(ulid))
            } else {
                PipewireReceiver::DefaultChanged(class, NodeTarget::UnmanagedNode(node_id))
            };
            let _ = self.callback_tx.send(message);
        }
    }

    pub fn find_default_source_id(&mut self) -> bool {
        self.populate_default_node_ids(false)
    }

    pub fn find_default_sink_id(&mut self) -> bool {
        self.populate_default_node_ids(true)
    }

    fn populate_default_node_ids(&mut self, is_sink: bool) -> bool {
        // Grab the Device Names
        let device = match is_sink {
            true => &mut self.default_sink,
            false => &mut self.default_source,
        };
        let configured = device.get_configured().map(|s| s.to_string());
        let default = device.get_default().map(|s| s.to_string());

        // Try to find and set the configured device node
        let mut send_update = false;
        if let Some(configured) = configured
            && let Some(id) = self.find_node_by_name(&configured)
        {
            let device = match is_sink {
                true => &mut self.default_sink,
                false => &mut self.default_source,
            };
            if device.set_configured_node_id(id) {
                send_update = true;
            }
        }

        // Try to find and set the default device node
        if let Some(default) = default
            && let Some(id) = self.find_node_by_name(&default)
        {
            let device = match is_sink {
                true => &mut self.default_sink,
                false => &mut self.default_source,
            };
            if device.set_default_node_id(id) {
                send_update = true;
            }
        }
        send_update
    }

    fn find_node_by_name(&self, name: &str) -> Option<u32> {
        for node in self.managed_nodes.values() {
            if let Some(node_name) = node.props.get("node.name")
                && node_name == name
            {
                return node.pw_id;
            }
        }
        for (id, node) in &self.unmanaged_device_nodes {
            if let Some(node_name) = &node.name
                && node_name == name
            {
                return Some(*id);
            }
        }
        None
    }

    pub fn set_default_sink_node(&self, target: NodeTarget) -> Result<()> {
        if !self.is_valid_target(target, MediaClass::Sink) {
            bail!("Target {:?} is not a valid sink", target);
        }

        let name = self
            .find_node_name_by_target(target)
            .ok_or_else(|| anyhow!("Node not found for target {:?}", target))?;
        let session = self
            .session_proxy
            .as_ref()
            .ok_or_else(|| anyhow!("No session proxy available"))?;

        session.metadata.set_property(
            0,
            "default.configured.audio.sink",
            Some("Spa:String:JSON"),
            Some(&format!(r#"{{"name":"{}"}}"#, name)),
        );
        Ok(())
    }

    pub fn set_default_source_node(&self, target: NodeTarget) -> Result<()> {
        if !self.is_valid_target(target, MediaClass::Source) {
            bail!("Target {:?} is not a valid source", target);
        }

        let name = self
            .find_node_name_by_target(target)
            .ok_or_else(|| anyhow!("Node not found for target {:?}", target))?;
        let session = self
            .session_proxy
            .as_ref()
            .ok_or_else(|| anyhow!("No session proxy available"))?;

        session.metadata.set_property(
            0,
            "default.configured.audio.source",
            Some("Spa:String:JSON"),
            Some(&format!(r#"{{"name":"{}"}}"#, name)),
        );
        Ok(())
    }

    pub fn is_valid_target(&self, target: NodeTarget, class: MediaClass) -> bool {
        let media_class = match target {
            NodeTarget::Node(ulid) => self
                .managed_nodes
                .get(&ulid)
                .and_then(|n| n.props.get(*MEDIA_CLASS))
                .map(|s| s.to_string()),
            NodeTarget::UnmanagedNode(id) => self
                .unmanaged_device_nodes
                .get(&id)
                .and_then(|n| n.media_class.clone()),
        };

        media_class.is_some_and(|c| match class {
            MediaClass::Sink => c.contains("Sink") || c.contains("Duplex"),
            MediaClass::Source => c.contains("Source") || c.contains("Duplex"),
            MediaClass::Duplex => c.contains("Duplex"),
        })
    }

    fn find_node_name_by_target(&self, target: NodeTarget) -> Option<String> {
        match target {
            NodeTarget::Node(ulid) => {
                let node = self.managed_nodes.get(&ulid)?;
                node.props.get("node.name").map(|s| s.to_string())
            }
            NodeTarget::UnmanagedNode(id) => self.unmanaged_device_nodes.get(&id)?.name.clone(),
        }
    }
}
