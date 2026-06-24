use anyhow::{Result, bail};

use crate::store::Store;
use log::debug;
use pipewire::device::{Device, DeviceChangeMask, DeviceListener};
use pipewire::keys::{DEVICE_DESCRIPTION, DEVICE_NAME, DEVICE_NICK, OBJECT_SERIAL};
use pipewire::registry::{GlobalObject, Registry};
use pipewire::spa::param::ParamType;
use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::pod::{Pod, Property, Value, ValueArray, object};
use pipewire::spa::sys::{
    SPA_PARAM_ROUTE_device, SPA_PARAM_ROUTE_index, SPA_PARAM_ROUTE_props, SPA_PARAM_ROUTE_save,
    SPA_PARAM_Route, SPA_PROP_channelVolumes, SPA_PROP_mute,
};
use pipewire::spa::utils;
use pipewire::spa::utils::dict::DictRef;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Cursor;
use std::rc::{Rc, Weak};

#[allow(unused)]
pub(crate) struct RegistryDevice {
    object_serial: u32,

    nickname: Option<String>,
    description: Option<String>,
    name: Option<String>,

    pub(crate) _proxy: Option<Device>,
    pub(crate) _listener: Option<DeviceListener>,

    pub(crate) nodes: Vec<u32>,
    pub(crate) active_routes: HashMap<u32, ActiveRoute>,
}

#[derive(Debug)]
pub(crate) struct ActiveRoute {
    pub index: u32,
    pub n_channels: u32,
}

impl From<&DictRef> for RegistryDevice {
    fn from(value: &DictRef) -> Self {
        let object_serial = value
            .get(*OBJECT_SERIAL)
            .and_then(|s| s.parse::<u32>().ok())
            .expect("OBJECT_SERIAL");
        let nickname = value.get(*DEVICE_NICK).map(|s| s.to_string());
        let description = value.get(*DEVICE_DESCRIPTION).map(|s| s.to_string());
        let name = value.get(*DEVICE_NAME).map(|s| s.to_string());

        Self {
            object_serial,
            nickname,
            description,
            name,
            _proxy: None,
            _listener: None,

            nodes: vec![],
            active_routes: HashMap::new(),
        }
    }
}

impl RegistryDevice {
    pub fn add_node(&mut self, id: u32) {
        self.nodes.push(id);
    }

    pub fn set_volume(
        &self,
        device_id: u32,
        route_index: u32,
        n_channels: u32,
        volume: f32,
    ) -> Result<()> {
        let Some(ref proxy) = self._proxy else {
            bail!("Proxy not found")
        };

        let pod = Value::Object(object! {
            utils::SpaTypes::ObjectParamRoute,
            ParamType::Route,
            Property::new(SPA_PARAM_ROUTE_index,  Value::Int(route_index as i32)),
            Property::new(SPA_PARAM_ROUTE_device, Value::Int(device_id as i32)),
            Property::new(SPA_PARAM_ROUTE_props,  Value::Object(object! {
                utils::SpaTypes::ObjectParamProps,
                ParamType::Props,
                Property::new(SPA_PROP_channelVolumes, Value::ValueArray(ValueArray::Float(vec![volume; n_channels as usize]))),
            })),
            Property::new(SPA_PARAM_ROUTE_save, Value::Bool(true)),
        });

        let (cursor, _) = PodSerializer::serialize(Cursor::new(Vec::new()), &pod)?;
        if let Some(pod) = Pod::from_bytes(&cursor.into_inner()) {
            proxy.set_param(ParamType::Route, 0, pod);
        }

        Ok(())
    }

    #[allow(unused)]
    pub fn set_mute(&self, device_id: u32, route_index: u32, mute: bool) -> Result<()> {
        let Some(ref proxy) = self._proxy else {
            bail!("Proxy not found")
        };

        let pod = Value::Object(object! {
            utils::SpaTypes::ObjectParamRoute,
            ParamType::Route,
            Property::new(SPA_PARAM_ROUTE_index,  Value::Int(route_index as i32)),
            Property::new(SPA_PARAM_ROUTE_device, Value::Int(device_id as i32)),
            Property::new(SPA_PARAM_ROUTE_props,  Value::Object(object! {
                utils::SpaTypes::ObjectParamProps,
                ParamType::Props,
                Property::new(SPA_PROP_mute, Value::Bool(mute)),
            })),
            Property::new(SPA_PARAM_ROUTE_save, Value::Bool(true)),
        });

        let (cursor, _) = PodSerializer::serialize(Cursor::new(Vec::new()), &pod)?;
        if let Some(pod) = Pod::from_bytes(&cursor.into_inner()) {
            proxy.set_param(ParamType::Route, 0, pod);
        }

        Ok(())
    }
}

pub fn handle_device(
    id: u32,
    global: &GlobalObject<&DictRef>,
    registry: Rc<RefCell<Registry>>,
    store: &mut Store,
    listener_store: Weak<RefCell<Store>>,
) {
    //let mut store = listener_store.borrow_mut();

    if let Some(props) = global.props {
        let mut device = RegistryDevice::from(props);
        let bound: Option<Device> = registry.borrow().bind(global).ok();

        if let Some(proxy) = bound {
            let info_local = listener_store.clone();
            let param_local = listener_store.clone();
            let listener = proxy
                .add_listener_local()
                .info(move |info| {
                    // For some reason, subscribe_params doesn't seem to work, so instead
                    // we'll listen for param changes, then call an enum to fetch the changes
                    if info.change_mask().contains(DeviceChangeMask::PARAMS)
                        && let Some(store) = info_local.upgrade()
                        && let Some(dev) = store.borrow().unmanaged_devices.get(&id)
                        && let Some(proxy) = &dev._proxy
                    {
                        proxy.enum_params(0, Some(ParamType::Route), 0, u32::MAX);
                    }
                })
                .param(move |_seq, _type, _index, _next, param| {
                    let Some(pod) = param else { return };
                    let Ok((_, Value::Object(obj))) =
                        PodDeserializer::deserialize_any_from(pod.as_bytes())
                    else {
                        return;
                    };

                    if obj.id != SPA_PARAM_Route {
                        return;
                    }

                    let mut route_index: Option<u32> = None;
                    let mut route_device: Option<u32> = None;
                    let mut n_channels: u32 = 2;
                    let mut current_volume: Option<u8> = None;
                    let mut current_mute: Option<bool> = None;

                    for prop in &obj.properties {
                        let key = prop.key;

                        if key == SPA_PARAM_ROUTE_index {
                            if let Value::Int(v) = prop.value {
                                route_index = Some(v as u32);
                            }
                        } else if key == SPA_PARAM_ROUTE_device {
                            if let Value::Int(v) = prop.value {
                                route_device = Some(v as u32);
                            }
                        } else if key == SPA_PARAM_ROUTE_props
                            && let Value::Object(props_obj) = &prop.value
                        {
                            if let Some(p) = props_obj
                                .properties
                                .iter()
                                .find(|p| p.key == SPA_PROP_channelVolumes)
                                && let Value::ValueArray(ValueArray::Float(vols)) = &p.value
                            {
                                n_channels = vols.len().max(1) as u32;
                                current_volume = vols
                                    .iter()
                                    .copied()
                                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                                    .map(|vol| (vol.cbrt() * 100.0).round() as u8);
                            }

                            if let Some(p) =
                                props_obj.properties.iter().find(|p| p.key == SPA_PROP_mute)
                                && let Value::Bool(muted) = p.value
                            {
                                current_mute = Some(muted);
                            }
                        }
                    }
                    if let (Some(route_index), Some(route_dev)) = (route_index, route_device)
                        && let Some(store) = param_local.upgrade()
                    {
                        let mut store = store.borrow_mut();

                        let route_changed = store
                            .unmanaged_devices
                            .get(&id)
                            .and_then(|d| d.active_routes.get(&route_dev))
                            .map(|r| r.index != route_index || r.n_channels != n_channels)
                            .unwrap_or(true);

                        if route_changed {
                            store.unmanaged_device_set_active_route(
                                id,
                                route_dev,
                                route_index,
                                n_channels,
                            );
                        }

                        if let Some(volume) = current_volume {
                            store.unmanaged_device_node_volume_changed(id, route_dev, volume);
                        }
                        if let Some(mute) = current_mute {
                            store.unmanaged_device_node_mute_changed(id, route_dev, mute);
                        }
                    }
                })
                .register();

            proxy.enum_params(0, Some(ParamType::Route), 0, u32::MAX);

            device._proxy = Some(proxy);
            device._listener = Some(listener);
        }

        store.unmanaged_device_add(id, device);
    }
}
