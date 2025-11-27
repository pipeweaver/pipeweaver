<script>
import PopupBox from "@/views/desktop/inputs/PopupBox.vue";
import {DeviceType, get_devices} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";

export default {
  components: {PopupBox},
  name: "ApplicationNodes",

  props: {
    isSource: {type: Boolean, default: true},
    nodes: {type: Array, required: true},
  },

  methods: {
    get_target_devices() {
      return get_devices(DeviceType.VirtualTarget)
    },
    get_source_devices() {
      return get_devices(DeviceType.VirtualSource)
    },
    get_devices() {
      return this.isSource ? this.get_source_devices() : this.get_target_devices();
    },

    get_target(node) {
      // Check for a default target
      if (!node.target) {
        return "-1";
      }

      if (node.target["Managed"]) {
        return node.target["Managed"];
      } else {
        // We should give non-pipeweaver nodes as an option, but for now, default / unknown.
        return "-1";
      }
    },

    show(e) {
      this.$refs.popup.showDialog(e, this.id)
    },

    close() {
      this.$refs.popup.close();
    },

    onClosed(e) {
      this.$emit('closed', e);
    },

    set_node_target(node_id, index, target) {
      this.$refs.popups[index].close();

      // SetTransientApplicationRoute(node_id, target)
      let command = {
        "SetTransientApplicationRoute": [node_id, target]
      }

      if (target === "-1") {
        command = {
          "ClearTransientApplicationRoute": node_id
        }
      }
      websocket.send_command(command);
    },

    volume_changed(e, node_id) {
      let value = e.target.value;
      // SetApplicationVolume(u32, u8),
      let command = {
        "SetApplicationVolume": [node_id, parseInt(value)]
      }
      websocket.send_command(command);
    },

    is_muted(node_id) {
      let node = this.nodes.find(n => n.node_id === node_id);
      if (node) {
        return node.muted;
      }
      return false
    },

    toggle_mute(node_id) {
      let node = this.nodes.find(n => n.node_id === node_id);
      if (!node) {
        return;
      }

      //SetApplicationMute(u32, bool),
      let value = !node.muted;
      let command = {
        "SetApplicationMute": [node_id, value]
      }
      websocket.send_command(command);
    },

    open_selector(e, app, id) {
      const anchor = (e && e.currentTarget) ? e.currentTarget : (e && e.target) ? e.target : undefined;
      const event = Object.assign({}, e, {target: anchor});
      if (this.$refs.popups && this.$refs.popups[id]) {
        this.$refs.popups[id].showDialog(event, app, undefined, true);
      }
    },

    get_application_target_name(node) {
      let target = this.get_target(node);
      if (target === "-1") {
        return "Default";
      }
      const devices = this.get_devices();
      for (const device of devices) {
        if (String(device.description.id) === target) {
          return device.description.name;
        }
      }
    },
  },

  computed: {
    maxDeviceNameWidth() {
      const devices = this.get_devices();
      if (!devices.length) return '95px';

      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      ctx.font = '14px sans-serif';

      let maxWidth = 0;
      devices.forEach(device => {
        const width = ctx.measureText(device.description.name).width;
        maxWidth = Math.max(maxWidth, width);
      });

      if (maxWidth + 10 > 95) {
        return '95px';
      }

      return maxWidth + 'px';
    }
  }
}
</script>

<template>
  <PopupBox ref="popup" @closed="onClosed">
    <div class="global">
      <div v-for="(node, index) in nodes" class="entry">
        <PopupBox ref="popups">
          <div :class="{ 'drop-selected': this.get_target(node) === '-1' }"
               :style="{ 'min-width': `calc(${maxDeviceNameWidth} + 1px)` }" class="drop-entry"
               @click="set_node_target(node.node_id, index, '-1')">
            <span class="drop-title">Default</span>
          </div>

          <div v-for="device in get_devices()">
            <div
              :class="{ 'drop-selected': String(this.get_target(node)) === String(device.description.id) }"
              :style="{ 'min-width': `calc(${maxDeviceNameWidth} + 3px )` }"
              class="drop-entry"
              @click="set_node_target(node.node_id, index, device.description.id)">
              <span class="drop-title">{{ device.description.name }}</span>
            </div>
          </div>
        </PopupBox>

        <div class="title">{{ node.title }}</div>
        <div class="content">
          <div>
            <button>
              <span ref="mute">
                <button>
                  <span @click="toggle_mute(node.node_id)">
                    <font-awesome-icon v-if="is_muted(node.node_id)" :icon="['fas', 'volume-xmark']"
                                       style="color: #ff0000"/>
                    <font-awesome-icon v-else :icon="['fas', 'volume-high']"/>
                  </span>
                </button>
              </span>
            </button>
          </div>
          <div><input :value="node.volume" max="100" min="0" type="range"
                      @input="e => volume_changed(e, node.node_id)"/></div>

          <div :style="{ width: `calc(${maxDeviceNameWidth} + 25px)` }" class="selector">
            <div class="inner" @click="open_selector($event, node, index)">
              <span v-if="this.get_target(node) === '-1'">Default</span>
              <span v-else>{{ this.get_application_target_name(node) }}</span>
              <font-awesome-icon :icon="['fas', 'angle-down']" class="selector-icon"/>
            </div>
          </div>
        </div>
      </div>
    </div>
  </PopupBox>
</template>

<style scoped>
.global {
  min-width: 290px;
}

.entry .title {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  padding-bottom: 10px;
}

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

.entry {
  white-space: nowrap;
  padding: 10px 10px 10px 10px;
}

.entry:not(:last-child) {
  border-bottom: 3px solid #3b413f;
}

.selector {
  display: flex;
  align-items: center;
  justify-content: center;
}

.selector .inner {
  display: flex;
  width: 100%;
  align-items: center;
  justify-content: space-between;
  border: 1px solid #666;
  box-sizing: border-box;
}

.selector .inner:hover {
  background-color: #3b413f;
  cursor: pointer;
}

.selector .inner span {
  padding: 2px 5px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.selector .inner svg {
  padding-right: 5px;
}

.drop-selected {
  background-color: #214283;
}

.drop-title {
  white-space: nowrap;
}

.drop-entry {
  white-space: nowrap;
  padding: 4px 10px 4px 10px;
}

.drop-entry:hover {
  background-color: #49514e;
  cursor: pointer;
}

.drop-entry:not(:last-child) {
  border-bottom: 1px solid #3b413f;
}

</style>
