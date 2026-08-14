//! Small helpers with no state of their own, shared by more than one
//! submodule. Kept separate so it's obvious at a glance that nothing here
//! owns any part of `Store`'s data.

use super::Store;
use crate::MediaClass;

impl Store {
    // ----- UTILITY FUNCTIONS -----
    /// `pub(super)` (rather than private) because both `unmanaged_devices`
    /// and `unmanaged_clients` classify their nodes by in/out channel count
    /// using this same rule.
    pub(super) fn get_media_class(&self, in_count: usize, out_count: usize) -> Option<MediaClass> {
        // Return the Specific MediaClass based on Channel Count
        if (1..=2).contains(&in_count) && (out_count == 0) {
            return Some(MediaClass::Sink);
        } else if (1..=2).contains(&out_count) && in_count == 0 {
            return Some(MediaClass::Source);
        } else if (1..=2).contains(&in_count) && in_count == out_count {
            // This is a bit of an assumption really, but we have non-monitor ports on the
            // tail end, so a reasonable assumption.
            return Some(MediaClass::Duplex);
        }
        None
    }
}
