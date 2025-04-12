<script>
// A lot of this is copied for MuteTargetSelector, mostly because I'm lazy :D

import PopupBox from "@/components/inputs/PopupBox.vue";
import {
  get_devices,
  getFullTargetList,
  getSourcePhysicalDevices,
  getTargetPhysicalDevices,
  is_source
} from "@/pipecast/util.js";

export default {
  name: "PhysicalDeviceSelector",
  components: {PopupBox},

  props: {
    type: {type: String, required: true},
    index: {type: Number, required: true},
    id: {type: String, required: true},
  },

  methods: {
    getFullTargetList,

    show(e) {
      this.$refs.popup.showDialog(e, this.id)
    },

    getDevice() {
      return get_devices(this.type)[this.index];
    },

    getId: function () {
      return this.getDevice().description.id;
    },

    getDevices: function () {
      let devices = (is_source(this.type)) ? getSourcePhysicalDevices() : getTargetPhysicalDevices();
      let list = [];
      let mapped_node_ids = [];

      // Loop over attached devices, and check whether it's attached
      let attached = this.getDevice().attached_devices;

      for (const [index, value] of attached.entries()) {
        let device = undefined;
        if (value.name !== null) {
          device = devices.find(device => value.name === device.name);
        } else {
          device = devices.find(device => value.description === device.description);
        }

        let node_id = undefined;
        let name = undefined;
        let description = undefined;

        if (device !== undefined) {
          // We have a device, grab the details from there
          node_id = device.id;
          name = device.name;
          description = device.description;

          // Flag this as already found, so we don't repeat later
          mapped_node_ids.push(device.id);
        } else {
          // No device found, use the Profile config
          name = value.name;
          description = value.description;
        }

        list.push({
          node_id: node_id,
          config_id: index,
          name: name,
          description: description
        })
      }

      // Iterate over the devices, and add them if they're not already present
      for (let device of devices) {
        if (!mapped_node_ids.includes(device.id)) {
          list.push({
            node_id: device.id,
            config_id: undefined,
            name: device.name,
            description: device.description,
          })
        }
      }

      // Ok, split the list out by description and name
      const [with_desc, without_desc] = list.reduce(
        ([with_desc, without_desc], item) =>
          item.description !== undefined
            ? [[...with_desc, item], without_desc]
            : [with_desc, [...without_desc, item]],
        [[], []]
      );

      with_desc.sort((a, b) => a.description.localeCompare(b.description));
      without_desc.sort((a, b) => a.name.localeCompare(b.name));

      let final = [];
      final.push(...with_desc);
      final.push(...without_desc);

      return final;
    },

    isConfigDevice: function (device) {
      return device.config_id !== undefined;
    },

    isDevicePresent: function (device) {
      return device.node_id !== undefined;
    },

    getDeviceName: function (device) {
      if (device.description === undefined) {
        return device.name;
      }
      return device.description;
    },

    onClick: function (device) {
      // Ok, work out what we need to do here
      if (this.isConfigDevice(device)) {
        console.log("TODO: Remove Index: " + device.config_id + " from " + this.getId());
        return;
      }

      console.log("TODO: Attached Node " + device.node_id + " to " + this.getId());
    },

    onClosed(e) {
      this.$emit('closed', e);
    }
  }
}
</script>

<template>
  <PopupBox ref="popup" @closed="onClosed">
    <div class="entry" @click="">
      <span class="selected"></span>
      <span>Rename?</span>
    </div>
    <div class="separator"/>
    <div v-for="device of getDevices()"
         :class="{error: !isDevicePresent(device), entry: isDevicePresent(device)}"
         @click="e => onClick(device)">
        <span class="selected">
          <font-awesome-icon v-if="isConfigDevice(device)" :icon="['fas', 'check']"/>
        </span>
      <span class="title">
        <span v-if="!isDevicePresent(device)" class="not_connected"><b>[Not Connected]</b></span>
        <span>{{ getDeviceName(device) }}</span>
      </span>
    </div>
    <div class="separator"/>
    <div class="entry" @click="">
      <span class="selected"></span>
      <span>Remove?</span>
    </div>

  </PopupBox>
</template>

<style scoped>
.separator {
  height: 5px;
  background-color: #3b413f;
}

.selected {
  display: inline-block;
  width: 20px;
  padding: 0;
}

.error {
  white-space: nowrap;
  padding: 6px 20px 6px 6px;
  background-color: #291b1b;
}

.error:hover {
  cursor: pointer;
  background-color: #3e2929;
}

.title {
  white-space: nowrap;
}

.entry {
  white-space: nowrap;
  padding: 6px 20px 6px 6px;
}

.entry:hover {
  background-color: #49514e;
  cursor: pointer;
}

.entry:not(:last-child) {
  border-bottom: 1px solid #3b413f;
}

.not_connected {
  display: inline-block;
  margin-right: 10px;
}

</style>
