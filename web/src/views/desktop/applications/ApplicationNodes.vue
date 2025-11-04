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

    onClosed(e) {
      this.$emit('closed', e);
    },

    set_node_target(e, node_id) {
      // SetTransientApplicationRoute(node_id, target)
      let target = e.target.value;

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
    }
  }
}
</script>

<template>
  <PopupBox ref="popup" @closed="onClosed">
    <div class="global">
      <div v-for="node in nodes" class="entry">
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
          <div>
            <select @change="e => set_node_target(e, node.node_id)">
              <option :selected="this.get_target(node) === -1" value="-1">Default</option>
              <option v-for="device in get_devices()"
                      :selected="this.get_target(node) === device.description.id"
                      :value="device.description.id">
                {{ device.description.name }}
              </option>
            </select>
          </div>
        </div>
      </div>
    </div>
  </PopupBox>
</template>

<style scoped>
.global {
  min-width: 290px;
  max-width: 290px;
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
</style>
