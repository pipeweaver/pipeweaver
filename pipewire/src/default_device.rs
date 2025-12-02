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
    configured: Option<String>, // User's explicit preference
    default: Option<String>,    // Current default from session manager
}

impl DefaultDevice {
    pub(crate) fn set(&mut self, definition: DefaultDefinition) -> bool {
        match definition {
            DefaultDefinition::Configured(name) => self.set_configured(Some(name)),
            DefaultDefinition::Default(name) => self.set_default(Some(name)),
        }
    }

    pub(crate) fn get_configured(&self) -> Option<&String> {
        self.configured.as_ref()
    }

    pub(crate) fn set_configured(&mut self, name: Option<String>) -> bool {
        let changed = self.configured != name;
        self.configured = name;
        changed
    }

    pub(crate) fn get_default(&self) -> Option<&String> {
        self.default.as_ref()
    }

    pub(crate) fn set_default(&mut self, name: Option<String>) -> bool {
        let changed = self.default != name;
        self.default = name;
        changed
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum DefaultDefinition {
    Configured(String),
    Default(String),
}
