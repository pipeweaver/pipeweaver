use pipecast_pipewire::{FilterHandler, FilterProperty, FilterValue, MediaClass};
use tokio::sync::oneshot;
use ulid::Ulid;

/// A waker filter is simply a filter which will do nothing until a sample has been received from,
/// or sent to, a device. The reason for this type of filter is that sometimes if a device is
/// suspending (especially USB devices) it can take a second for them to run up and start sending
/// data.
///
/// If we attach a device before it's ready the latency caused by waiting for it to wake can be
/// propagated through the entire pipecast tree, forcing max latency on other nodes.
///
/// This filter will trigger a callback as soon as pipewire passes samples to the filter, which
/// should indicate full awakeness of the device.
pub struct WakerFilter {
    wake_for: Ulid,
    sender: Option<oneshot::Sender<Ulid>>,
    class: MediaClass,
}

impl WakerFilter {
    pub(crate) fn new(wake_for: Ulid, sender: oneshot::Sender<Ulid>, class: MediaClass) -> Self {
        if class == MediaClass::Duplex {
            panic!("Attempted to Create Waker as Full Duplex!");
        }

        Self {
            wake_for,
            sender: Some(sender),
            class,
        }
    }
}

impl FilterHandler for WakerFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![]
    }

    fn get_property(&self, _: u32) -> FilterProperty {
        panic!("Attempted to get non-existent property");
    }

    fn set_property(&mut self, _: u32, _: FilterValue) {
        panic!("Attempted to set non-existent property");
    }

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, outputs: Vec<&mut [f32]>) {
        // We've already triggered this, so don't do anything.
        if self.sender.is_none() {
            return;
        }

        let check = match self.class {
            MediaClass::Source => inputs,
            MediaClass::Sink => outputs,
            _ => {
                panic!("Unexpected Duplex in Waker!");
            }
        };

        if !check.is_empty() {
            let _ = self.sender.take().unwrap().send(self.wake_for);
        }
    }
}