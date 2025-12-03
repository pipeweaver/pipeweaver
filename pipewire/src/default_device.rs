// Ok, I'm adding new stuff, and figure I should actually do this properly, rather than just
// slapping everything into either the registry or hacking around it in the store...

/// Used to represent a pipewire default device, these come from the general metadata, and includes
/// two possible keys: 'default.audio.x' and 'default.audio.configured.x'. Neither are guarenteed
/// to be present, and the values may not map to valid nodes (configured can continue to exist
/// after node removal).
///
/// Messages come in via the Spa:String:JSON type, and look something like this:
/// { "name": "pipeweaver_system" }
///
/// Because the node id / node serial details aren't part of the message, and this data can be
/// received before the node tree has populated, we can only store the name as a String here,
/// and perform lookups later.
#[derive(Debug, Clone, Default)]
pub(crate) struct DefaultDevice {
    configured: Option<DeviceState>, // User's explicit preference
    default: Option<DeviceState>,    // Current default from session manager
}

#[derive(Debug, Clone)]
struct DeviceState {
    name: String,
    id: Option<u32>,
}

impl DefaultDevice {
    pub(crate) fn set(&mut self, definition: DefaultDefinition) -> bool {
        match definition {
            DefaultDefinition::Configured(name) => self.set_configured(name),
            DefaultDefinition::Default(name) => self.set_default(name),
        }
    }

    // Sorry, but the way that rustfmt works means that this looks ugly, so we'll skip
    // formatting in this case.
    pub(crate) fn get_active_node_id(&self) -> Option<u32> {
        match (&self.configured, &self.default) {
            (Some(node), _) if node.id.is_some() => node.id,
            (_, Some(node)) if node.id.is_some() => node.id,
            _ => None,
        }
    }

    pub(crate) fn get_configured(&self) -> Option<&String> {
        if let Some(configured) = &self.configured {
            return Some(&configured.name);
        }
        None
    }

    pub(crate) fn set_configured(&mut self, name: String) -> bool {
        // If this is the same as the existing, do nothing.
        if let Some(node) = &self.configured
            && node.name == name
        {
            return false;
        }

        self.configured = Some(DeviceState {
            name: name.clone(),
            id: None,
        });
        true
    }

    pub(crate) fn set_configured_node_id(&mut self, id: u32) -> bool {
        if let Some(node) = &mut self.configured {
            node.id = Some(id);
            return true;
        }
        false
    }

    pub(crate) fn get_default(&self) -> Option<&String> {
        if let Some(default) = &self.default {
            return Some(&default.name);
        }
        None
    }

    pub(crate) fn set_default(&mut self, name: String) -> bool {
        if let Some(node) = &self.configured
            && node.name == name
        {
            return false;
        }

        self.default = Some(DeviceState {
            name: name.clone(),
            id: None,
        });
        true
    }

    pub(crate) fn set_default_node_id(&mut self, id: u32) -> bool {
        if let Some(node) = &mut self.default {
            node.id = Some(id);

            return if let Some(configured) = &self.configured {
                // If configured.id is None, then this change is important to signal.
                configured.id.is_none()
            } else {
                // Configured is not set, so this is a change.
                true
            };
        }
        false
    }

    /// When a device is added, we can check if it matches either the configured or default
    /// and update the id accordingly.
    pub(crate) fn device_added(&mut self, id: u32, name: &str) -> bool {
        if let Some(node) = &mut self.configured
            && node.name == name
        {
            node.id = Some(id);

            // If we've changed this device, we should *ALWAYS* return true, this ensures
            // that upstream can find the new default node.
            return true;
        }

        if let Some(node) = &mut self.default
            && node.name == name
        {
            node.id = Some(id);

            return if let Some(configured) = &self.configured {
                // If configured.id is None, then this change is important to signal.
                configured.id.is_none()
            } else {
                // Configured is not set, so this is a change.
                true
            };
        }

        // Nothing Changed.
        false
    }

    /// When a device is removed, if it's configured as a default, we need to clear the
    /// node id.
    pub(crate) fn device_removed(&mut self, id: u32) -> bool {
        if let Some(node) = &mut self.configured
            && node.id == Some(id)
        {
            node.id = None;
            return true;
        }

        if let Some(node) = &mut self.default
            && node.id == Some(id)
        {
            node.id = None;

            return if let Some(configured) = &self.configured {
                // If configured.id is None, then this change is important to signal.
                configured.id.is_none()
            } else {
                // Configured is not set, so this is a change.
                true
            };
        }

        false
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum DefaultDefinition {
    Configured(String),
    Default(String),
}
