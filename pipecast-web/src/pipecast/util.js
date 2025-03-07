import {store} from "@/pipecast/store.js";

export const DeviceType = Object.freeze({
  PHYSICAL_SOURCE: Symbol(''),
  VIRTUAL_SOURCE: Symbol(''),

  PHYSICAL_TARGET: Symbol(''),
  VIRTUAL_TARGET: Symbol(''),
});

export function get_devices(type) {
  if (type === DeviceType.PHYSICAL_SOURCE) {
    return store.getProfile().devices.sources.physical_devices;
  }
  if (type === DeviceType.VIRTUAL_SOURCE) {
    return store.getProfile().devices.sources.virtual_devices;
  }
  if (type === DeviceType.PHYSICAL_TARGET) {
    return store.getProfile().devices.targets.physical_devices;
  }
  if (type === DeviceType.VIRTUAL_TARGET) {
    return store.getProfile().devices.targets.virtual_devices;
  }
}

export function is_source(type) {
  return (type === DeviceType.PHYSICAL_SOURCE || type === DeviceType.VIRTUAL_SOURCE)
}
