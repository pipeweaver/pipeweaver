//! Small helpers with no state of their own, shared by more than one
//! submodule. Kept separate so it's obvious at a glance that nothing here
//! owns any part of `Store`'s data.

use crate::MediaClass;
use pipewire::node::Node;
use pipewire::spa::param::ParamType;
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::pod::{Pod, Property, Value, ValueArray, object};
use pipewire::spa::sys::{SPA_PROP_channelVolumes, SPA_PROP_mute};
use pipewire::spa::utils;
use std::io::Cursor;

// ----- UTILITY FUNCTIONS -----
/// `pub(super)` (rather than private) because both `unmanaged_devices`
/// and `unmanaged_clients` classify their nodes by in/out channel count
/// using this same rule.
pub(crate) fn get_media_class(in_count: usize, out_count: usize) -> Option<MediaClass> {
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

/// Send Volume and Mute are common by proxy, so lives here so it can be shared between
/// Nodes, Devices and Applications.
pub(crate) fn send_volume(proxy: &Node, volume: u8) {
    let volume = (volume as f32 / 100.0).powi(3);
    let pod = Value::Object(object! {
        utils::SpaTypes::ObjectParamProps,
        ParamType::Props,
        Property::new(SPA_PROP_channelVolumes, Value::ValueArray(ValueArray::Float(vec![volume, volume]))),
    });

    let Ok((cursor, _)) = PodSerializer::serialize(Cursor::new(Vec::new()), &pod) else {
        return;
    };
    let bytes = cursor.into_inner();
    if let Some(bytes) = Pod::from_bytes(&bytes) {
        proxy.set_param(ParamType::Props, 0, bytes);
    }
}
pub(crate) fn send_mute(proxy: &Node, muted: bool) {
    let pod = Value::Object(object! {
        utils::SpaTypes::ObjectParamProps,
        ParamType::Props,
        Property::new(SPA_PROP_mute, Value::Bool(muted)),
    });

    let Ok((cursor, _)) = PodSerializer::serialize(Cursor::new(Vec::new()), &pod) else {
        return;
    };
    let bytes = cursor.into_inner();
    if let Some(bytes) = Pod::from_bytes(&bytes) {
        proxy.set_param(ParamType::Props, 0, bytes);
    }
}
