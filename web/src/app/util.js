import {store} from "@/app/store.js";

export const DeviceType = Object.freeze({
  PhysicalSource: 'PhysicalSource',
  VirtualSource: 'VirtualSource',
  PhysicalTarget: 'PhysicalTarget',
  VirtualTarget: 'VirtualTarget',
});

export const DeviceOrderType = Object.freeze({
  Default: 'Default', Pinned: 'Pinned', Hidden: 'Hidden',
})

export function get_device_order(order_type, is_source) {
  if (store.getProfile().devices === undefined) {
    return [];
  }

  if (is_source) {
    return store.getProfile().devices.sources.device_order[order_type];
  } else {
    return store.getProfile().devices.targets.device_order[order_type];
  }
}

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

export function get_device_by_id(id) {
  if (store.getProfile().devices === undefined) {
    return undefined;
  }

  // Iterate through all the device lists, and try to find this device
  for (let device of store.getProfile().devices.sources.physical_devices) {
    if (device.description.id === id) {
      return device;
    }
  }

  for (let device of store.getProfile().devices.sources.virtual_devices) {
    if (device.description.id === id) {
      return device;
    }
  }

  for (let device of store.getProfile().devices.targets.physical_devices) {
    if (device.description.id === id) {
      return device;
    }
  }

  for (let device of store.getProfile().devices.targets.virtual_devices) {
    if (device.description.id === id) {
      return device;
    }
  }
}

export function get_device_type(id) {
  if (store.getProfile().devices === undefined) {
    return [];
  }

  // Iterate through all the device lists, and try to find this device
  for (let device of store.getProfile().devices.sources.physical_devices) {
    if (device.description.id === id) {
      return DeviceType.PhysicalSource;
    }
  }

  for (let device of store.getProfile().devices.sources.virtual_devices) {
    if (device.description.id === id) {
      return DeviceType.VirtualSource;
    }
  }

  for (let device of store.getProfile().devices.targets.physical_devices) {
    if (device.description.id === id) {
      return DeviceType.PhysicalTarget;
    }
  }

  for (let device of store.getProfile().devices.targets.virtual_devices) {
    if (device.description.id === id) {
      return DeviceType.VirtualTarget;
    }
  }
}

export function is_source(type) {
  return (type === DeviceType.PhysicalSource || type === DeviceType.VirtualSource)
}

export function is_physical(type) {
  return (type === DeviceType.PhysicalTarget || type === DeviceType.PhysicalSource);
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
      id: device.description.id, name: device.description.name,
    });
  }
  return list;
}

export function getSourcePhysicalDevices() {
  return store.getAudio().devices.Source;
}

export function getTargetPhysicalDevices() {
  return store.getAudio().devices.Target;
}
