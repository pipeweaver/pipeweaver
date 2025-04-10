import {store} from "@/pipecast/store.js";

export const DeviceType = Object.freeze({
  PhysicalSource: 'PhysicalSource',
  VirtualSource: 'VirtualSource',

  PhysicalTarget: 'PhysicalTarget',
  VirtualTarget: 'VirtualTarget',
});

export function get_devices(type) {
  if (store.getProfile().devices === undefined) {
    return [];
  }

  if (type === DeviceType.PhysicalSource) {
    return store.getProfile().devices.sources.physical_devices;
  }
  if (type === DeviceType.VirtualSource) {
    return store.getProfile().devices.sources.virtual_devices;
  }
  if (type === DeviceType.PhysicalTarget) {
    return store.getProfile().devices.targets.physical_devices;
  }
  if (type === DeviceType.VirtualTarget) {
    return store.getProfile().devices.targets.virtual_devices;
  }
}

export function is_source(type) {
  return (type === DeviceType.PhysicalSource || type === DeviceType.VirtualSource)
}

// Some functions useful for getting basic node data
export function getFullSourceList() {
  let list = getNamesForDevices(get_devices(DeviceType.PhysicalSource));
  return list.concat(getNamesForDevices(get_devices(DeviceType.VirtualSource)));
}

export function getFullTargetList() {
  let list = getNamesForDevices(get_devices(DeviceType.PhysicalTarget));
  return list.concat(getNamesForDevices(get_devices(DeviceType.VirtualTarget)));
}

export function getNamesForDevices(devices) {
  let list = [];
  for (let device of devices) {
    list.push({
      id: device.description.id,
      name: device.description.name,
    });
  }
  return list;
}
