<script>
import ProcessList from "@/views/desktop/applications/ProcessList.vue";
import {store} from "@/app/store.js";
import {DeviceType, get_device_by_id, get_devices} from "@/app/util.js";
import PopupBox from "@/views/desktop/inputs/PopupBox.vue";
import {websocket} from "@/app/sockets.js";

export default {
  name: "applications",
  components: {PopupBox, ProcessList},

  methods: {
    getDefaultDevice(direction) {
      this.getDeviceList(direction);

      const audio = store.getAudio();

      // New Code, Pull from the defaults_id set
      const device = audio.defaults_id[direction];

      if (!device) {
        return "Device Not Set (This is a Bug!)";
      }

      if (get_device_by_id(device) !== undefined) {
        const node = get_device_by_id(device);
        return "PipeWeaver " + node?.description?.name ?? "Unknown device (This is a Bug!)";
      }

      const node = audio.devices[direction].find(dev => dev.id === device);
      if (!node) {
        return "Unknown device (This is a Bug!)";
      }

      return node.description;
    },

    getDefaultDeviceId(direction) {
      return store.getAudio().defaults_id[direction];
    },

    setDefaultDevice(index, direction, id) {
      this.$refs['popups'][index].close();

      let command = undefined;
      if (direction === "Source") {
        command = {
          "SetDefaultInput": id
        }
      } else if (direction === "Target") {
        command = {
          "SetDefaultOutput": id
        }
      }
      websocket.send_command(command);
    },

    getDeviceList(direction) {
      const audio = store.getAudio();
      const virtualType = direction === "Source" ? DeviceType.VirtualTarget : DeviceType.VirtualSource;

      const list = {};

      for (let node of audio.devices[direction]) {
        list[node.id] = node.description ?? node.name;
      }

      for (let device of get_devices(virtualType)) {
        list[device.description.id] = "PipeWeaver " + device.description.name;
      }

      return Object.fromEntries(
        Object.entries(list).sort(([, a], [, b]) => a.localeCompare(b))
      );
    },

    open_selector(e, type) {
      // The dialog only needs the target element to position itself, so we'll get the element
      // attaches to the event and pass that along instead of the raw event.
      const anchor = (e && e.currentTarget) ? e.currentTarget : (e && e.target) ? e.target : undefined;
      const event = Object.assign({}, e, {target: anchor});

      console.log(this.$refs.popups[type]);

      if (this.$refs.popups && this.$refs.popups[type]) {
        // pass the anchor element (instead of the raw target)
        this.$refs.popups[type].showDialog(event, undefined, undefined, true);
      }
    },
  },

  computed: {
    maxDeviceNameWidth() {
      return (direction) => {
        const devices = this.getDeviceList(direction);
        const deviceList = Object.values(devices);

        console.log(deviceList.length);
        if (!deviceList.length) return '95px';

        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');
        ctx.font = '14px sans-serif';

        let maxWidth = 0;
        deviceList.forEach(device => {
          const width = ctx.measureText(device).width + 5;
          console.log("Measured as " + width);
          maxWidth = Math.max(maxWidth, width);
        });

        console.log("maxWidth: " + maxWidth);
        return maxWidth + 'px';
      };
    }
  }
}
</script>

<template>
  <div class="applications">
    <div class="defaults">
      <div style="padding-bottom: 6px;"><b>Default Devices</b></div>
      <div v-for="(type, index) in ['Target', 'Source']">
        <PopupBox ref="popups">
          <div v-for="(name, key) in getDeviceList(type)">
            <div
              :class="{ 'selected': key === getDefaultDeviceId(type) }"
              :style="{ 'min-width': `calc(${maxDeviceNameWidth(type)} + 3px)` }" class="entry"
              @click="setDefaultDevice(index, type, key)">
              <span class="title">{{ name }}</span>
            </div>
          </div>
        </PopupBox>
        <div class="wrapper">
          <div class="name">{{ type === "Source" ? "Input:" : "Output:" }}</div>
          <div class="selector">
            <div class="inner" @click="open_selector($event, index)">
              <span>{{ getDefaultDevice(type) }}</span>
              <font-awesome-icon :icon="['fas', 'angle-down']" class="selector-icon"/>
            </div>
          </div>
        </div>
      </div>
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
}

.defaults .selector {
  display: flex;
  align-items: center;
  justify-content: center;

  min-width: initial;
}

.defaults .selector .inner {
  display: flex;
  width: 100%;
  max-width: 240px;
  min-width: 240px;
  align-items: center;
  justify-content: space-between;
  border: 1px solid #666;
  box-sizing: border-box;
}

.defaults .selector .inner:hover {
  background-color: #3b413f;
  cursor: pointer;
}

.defaults .selector .inner span {
  max-width: 250px;
  white-space: nowrap;
  padding: 5px 5px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.defaults .selector .inner svg {
  padding-right: 5px;
}


.wrapper {
  display: flex;
  flex-direction: row;

  align-items: center;
  justify-content: center;

  margin-bottom: 8px;
}

.wrapper .name {
  width: 60px;
  min-width: initial;
  font-weight: bold;
}

.defaults {
  max-width: initial;
}

.selected {
  background-color: #214283;
}

.entry {
  white-space: nowrap;
  padding: 4px 10px 4px 10px;
}

.entry:hover {
  background-color: #49514e;
  cursor: pointer;
}

.entry:not(:last-child) {
  border-bottom: 1px solid #3b413f;
}

.title {
  white-space: nowrap;
}
</style>
