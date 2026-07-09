<script>
import {get_devices} from "@/app/util.js";
import {store} from "@/app/store.js";
import {websocket} from "@/app/sockets.js";

export default {
  name: "PhysicalDeviceList",
  props: {
    isSource: {type: Boolean, default: true},
  },

  methods: {
    get_nodes() {
      let devices = store.getDevices();
      let devs = this.isSource ? devices["Source"] : devices["Target"];

      return devs.toSorted((a, b) =>
        this.get_name(a).localeCompare(this.get_name(b))
      );
    },

    get_name(dev) {
      return dev.description ?? dev.name
    },

    toggle_mute(dev) {
      let new_mute = !dev.muted;
      // SetDeviceMute(u32, bool),
      let command = {
        "SetPhysicalDeviceMute": [dev.id, new_mute]
      }
      websocket.send_command(command);
    },

    set_volume(e, dev) {
      // SetDeviceVolume(u32, u8),
      let value = e.target.value;
      let command = {
        "SetPhysicalDeviceVolume": [dev.id, parseInt(value)]
      }
      websocket.send_command(command);
    },
  }
}
</script>

<template>
  <div v-for="dev in get_nodes()">
    <div class="title">{{ get_name(dev) }}</div>
    <div ref="controls" class="content">
      <div>
        <button>
                <span ref="mute" :class="{ 'muted': dev.muted }">
                  <button>
                    <span @click="toggle_mute(dev)">
                      <font-awesome-icon v-if="dev.muted" :icon="['fas', 'volume-xmark']"/>
                      <font-awesome-icon v-else :icon="['fas', 'volume-high']"/>
                    </span>
                  </button>
                </span>
        </button>
      </div>
      <div>
        <input :value="dev.volume" max="100" min="0" type="range"
               @input="e => set_volume(e, dev)"/></div>
    </div>
  </div>
</template>

<style scoped>
.content {
  display: flex;
  flex-direction: row;
  height: 20px;
}

.content input[type=range] {
  width: 160px;
  margin-right: 5px;
}

.content button {
  all: unset;
  width: 20px;
  height: 20px;
}

.content .muted {
  color: #ff0000;
}

.content button span {
  display: inline-block;
}

.content select {
  min-width: 80px;
  max-width: 80px;

  border: 0;
  padding: 0 5px;

  height: 20px;
}
</style>
