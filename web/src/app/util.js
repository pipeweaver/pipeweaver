import {store} from "@/app/store.js";
import {onMounted, onUnmounted, ref} from "vue";

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
export function getFullSourceList(include_hidden) {
  return getOrderedList(true, include_hidden);
}

export function getFullTargetList(include_hidden) {
  return getOrderedList(false, include_hidden);
}

function getOrderedList(is_source, include_hidden) {
  // We should order this based on the general ordering
  let pinned = get_device_order(DeviceOrderType.Pinned, is_source);
  let base = get_device_order(DeviceOrderType.Default, is_source);


  let list = [];
  for (const item of pinned) {
    list.push(getNameForDevice(get_device_by_id(item)));
  }

  for (const item of base) {
    list.push(getNameForDevice(get_device_by_id(item)));
  }

  if (include_hidden) {
    let hidden = get_device_order(DeviceOrderType.Hidden, is_source);
    for (const item of hidden) {
      list.push(getNameForDevice(get_device_by_id(item)));
    }
  }

  return list;
}

export function getNameForDevice(device) {
  return {
    id: device.description.id, name: device.description.name,
  }
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
  return store.getAudio().devices.Source.filter(device => device.is_usable);
}

export function getTargetPhysicalDevices() {
  return store.getAudio().devices.Target.filter(device => device.is_usable);
}


export function useDeviceType() {
  const isDesktop = ref(true);

  onMounted(() => {
    const ua = navigator.userAgent;
    const isMobileOrTablet = /tablet|ipad|playbook|silk|(android(?!.*mobi))|Mobile|Android|iP(hone|od)|IEMobile|BlackBerry|Kindle|Silk-Accelerated|(hpw|web)OS|Fennec|Opera Mini/i.test(ua);
    isDesktop.value = !isMobileOrTablet;
  });

  return {isDesktop};
}

export function useOrientation() {
  const isPortrait = ref(window.matchMedia('(orientation: portrait)').matches);

  const update = () => {
    isPortrait.value = window.matchMedia('(orientation: portrait)').matches;
  };

  onMounted(() => {
    window.addEventListener('resize', update);
    window.addEventListener('orientationchange', update);
    update();
  });

  onUnmounted(() => {
    window.removeEventListener('resize', update);
    window.removeEventListener('orientationchange', update);
  });

  return {isPortrait};
}
