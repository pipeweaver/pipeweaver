//! Lifecycle of the filters we create (e.g. volume/mute processing nodes
//! implemented as pipewire filters rather than plain nodes), and access to
//! their runtime parameters.

use super::{ManagedFilter, Store};
use crate::{FilterProperty, FilterValue, LinkType};
use anyhow::{Result, anyhow};
use log::debug;
use ulid::Ulid;

impl Store {
    // ----- MANAGED FILTERS -----
    pub fn add_pending_filter(&mut self, seq: i32, id: Ulid) {
        self.pending_filter_syncs.insert(seq, id);
    }

    pub fn resolve_pending_filter_sync(&mut self, id: Ulid) {
        let filter = self.managed_filters.get_mut(&id).expect("Broke");
        if let Some(Some(sender)) = filter.ready_sender.take() {
            let _ = sender.send(());
        }
    }

    pub fn managed_filter_add(&mut self, filter: ManagedFilter) {
        debug!("[{}] Filter Added to Store", filter.id);
        self.managed_filters.insert(filter.id, filter);
    }

    pub fn managed_filter_get(&self, id: Ulid) -> Option<&ManagedFilter> {
        self.managed_filters.get(&id)
    }

    pub fn managed_filter_remove(&mut self, filter: Ulid) {
        self.managed_link_remove_for_type(LinkType::Filter(filter));
        self.managed_filters.remove(&filter);
    }

    pub fn managed_filter_set_pw_id(&mut self, id: Ulid, pw_id: u32) {
        let filter = self.managed_filters.get_mut(&id).expect("Broke");
        filter.pw_id = Some(pw_id);
    }

    pub fn managed_filter_set_parameter(
        &mut self,
        id: Ulid,
        key: u32,
        value: FilterValue,
    ) -> Result<String> {
        // Find the filter
        let filter = self
            .managed_filters
            .get_mut(&id)
            .ok_or(anyhow!("Filter Not Found"))?;

        // Set the Property
        filter.data.write().callback.set_property(key, value)
    }

    pub fn managed_filter_get_parameters(&self, id: Ulid) -> Result<Vec<FilterProperty>> {
        // Find the filter
        let filter = self
            .managed_filters
            .get(&id)
            .ok_or(anyhow!("Filter Missing"))?;

        // Send the Properties
        Ok(filter.data.read().callback.get_properties())
    }
}
