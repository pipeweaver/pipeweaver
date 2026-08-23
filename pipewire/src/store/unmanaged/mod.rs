use crate::registry::link::RegistryLink;
use crate::store::Store;
use std::collections::HashMap;

pub(super) mod client;
pub(super) mod device;

impl Store {
    // ----- UNMANAGED LINKS -----
    pub fn unmanaged_link_add(&mut self, id: u32, link: RegistryLink) {
        let in_pending = self
            .pending_link_syncs
            .iter()
            .any(|p| p.bound_ids.values().any(|&pw_id| pw_id == id));

        // Check our Managed Links to see if this is actually unmanaged
        if self.is_managed_link(id).is_none() && !in_pending {
            self.unmanaged_links.insert(id, link);
        }
    }

    pub fn unmanaged_link_remove(&mut self, id: u32) {
        self.unmanaged_links.remove(&id);
    }

    pub fn get_unmanaged_links(&self) -> &HashMap<u32, RegistryLink> {
        &self.unmanaged_links
    }
}
