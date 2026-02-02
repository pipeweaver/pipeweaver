<script>
import ProcessList from "@/views/desktop/applications/ProcessList.vue";
import {store} from "@/app/store.js";
import {get_device_by_id} from "@/app/util.js";

export default {
  name: "applications",
  components: {ProcessList},

  methods: {
    getDefaultDevice(direction) {
      const audio = store.getAudio();
      const device = audio.defaults[direction];

      if (!device) {
        return "Device Not Set (This is a Bug!)";
      }

      if (device.Managed) {
        const node = get_device_by_id(device.Managed);
        return node?.description?.name ?? "Unknown device (This is a Bug!)";
      }

      const nodeId = device.Unmanaged;
      const node = audio.devices[direction].find(dev => dev.node_id === nodeId);

      if (!node) {
        return "Unknown device (This is a Bug!)";
      }

      return node.description;
    },

    getDefaultOutputDevice() {
      let device = store.getAudio().defaults['Target'];
      if (!device) {
        return "No default Input device";
      }

      if (device['Managed']) {
        let node = get_device_by_id(device['Managed']);
        return node.description.name;
      }

      // Ok, we should have an unmanaged device here.
      let node_id = device['Unmanaged'];
      let node = store.getAudio().devices.Target.find(dev => dev.node_id === node_id);
      if (!node) {
        return "Unknown device (This is a Bug!)";
      }

      return node.description;
    },

    getDefaultInputDevice() {
      let device = store.getAudio().defaults['Source'];
      if (!device) {
        return "No default output device";
      }

      if (device['Managed']) {
        // We need to find the managed name for this device..
        let node = get_device_by_id(device['Managed']);
        return node.description.name;
      }

      let node_id = device['Unmanaged'];
      let node = store.getAudio().devices.Source.find(dev => dev.node_id === node_id);
      if (!node) {
        return "Unknown device (This is a Bug!)";
      }

      return node.description;
    }

  }
}
</script>

<template>
  <div class="applications">
    <div>
      <b>Default Devices</b><br/>
      <b>Output: </b> {{ getDefaultDevice("Target") }}<br/>
      <b>Input: </b> {{ getDefaultDevice("Source") }}<br/>
    </div>
    <div>
      <b>Output</b>
      <ProcessList :is-source="true"/>
    </div>
    <div>
      <b>Input</b>
      <ProcessList :is-source="false"/>
    </div>
  </div>
</template>

<style scoped>
.applications {
  padding: 10px;
  min-height: 200px;
  display: flex;
  flex-direction: row;
  gap: 25px;
}

.applications div {
  min-width: 300px;
  max-width: 300px;
}
</style>
