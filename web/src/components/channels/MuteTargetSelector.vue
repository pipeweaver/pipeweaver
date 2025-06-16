<script>
import PopupBox from "@/components/inputs/PopupBox.vue";
import {get_device_by_id, getFullTargetList} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";

export default {
  name: "MuteTargetSelector",
  components: {PopupBox},

  props: {
    type: {type: String, required: true},
    target: {type: String, required: true},
    device_id: {type: String, required: true},
    id: {type: String, required: true},
  },

  methods: {
    getFullTargetList,

    show(e) {
      this.$refs.popup.showDialog(e, this.id)
    },

    getDevice() {
      return get_device_by_id(this.device_id);
    },

    getId: function () {
      return this.getDevice().description.id;
    },

    isMutedAll() {
      let device = this.getDevice();
      return (device.mute_states.mute_targets[this.target].length === 0);
    },

    isMutedTo(id) {
      let device = this.getDevice();
      return (device.mute_states.mute_targets[this.target].includes(id));
    },

    setMuteToAll() {
      // ClearMuteTargetNodes(Ulid, MuteTarget),
      let command = {
        "ClearMuteTargetNodes": [this.getId(), this.target]
      }
      websocket.send_command(command);
    },

    toggleMuteToTarget(id) {
      // AddMuteTargetNode(Ulid, MuteTarget, Ulid),
      // DelMuteTargetNode(Ulid, MuteTarget, Ulid),

      let command_name = !this.isMutedTo(id) ? "AddMuteTargetNode" : "DelMuteTargetNode";
      let command = {
        [command_name]: [this.getId(), this.target, id]
      }
      websocket.send_command(command);
    },

    onClosed(e) {
      this.$emit('closed', e);
    }
  }
}
</script>

<template>
  <PopupBox ref="popup" @closed="onClosed">
    <div class="entry" @click="setMuteToAll()">
        <span class="selected">
          <font-awesome-icon v-if="isMutedAll()" :icon="['fas', 'check']"/>
        </span>
      <span>Mute to All</span>
    </div>
    <div class="separator"/>
    <div v-for="target of getFullTargetList(true)" class="entry"
         @click="toggleMuteToTarget(target.id)">
        <span class="selected">
          <font-awesome-icon v-if="isMutedTo(target.id)" :icon="['fas', 'check']"/>
        </span>
      <span class="title">Mute to {{ target.name }}</span>
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
</style>
