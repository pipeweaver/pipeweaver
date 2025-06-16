<script>
import ChannelColumn from "@/components/channels/ChannelColumn.vue";
import {DeviceOrderType, DeviceType, get_device_by_id, get_device_order} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";
import PopupBox from "@/components/inputs/PopupBox.vue";
import DeviceList from "@/components/DeviceList.vue";

const INTERNAL_SCALE = 0.8;

export default {
  name: "Mixer",
  components: {DeviceList, PopupBox, ChannelColumn},

  data() {
    return {}
  },

  props: {
    is_source: Boolean,
  },
  methods: {
    get_device_by_id,


    show_popup(e) {
      this.$refs.popup.showDialog(e)
    },

    show_hidden(e) {
      this.$refs.hidden.showDialog(e)
    },

    has_pinned() {
      return get_device_order(DeviceOrderType.Pinned, this.is_source).length > 0;
    },

    has_hidden() {
      return get_device_order(DeviceOrderType.Hidden, this.is_source).length > 0
    },

    get_hidden() {
      return get_device_order(DeviceOrderType.Hidden, this.is_source);
    },

    show_device(id) {
      this.$refs.hidden.hideDialog();
      let command = {
        "SetOrderGroup": [id, DeviceOrderType.Default],
      }
      websocket.send_command(command);
    },

    add_device(is_physical) {
      this.$refs.popup.hideDialog();
      let name = prompt("Device Name:");

      if (name === undefined || name === "" || name === null) {
        return;
      }

      // We need to break down the type
      let final_type = undefined;
      if (is_physical) {
        if (this.is_source) {
          final_type = DeviceType.PhysicalSource;
        } else {
          final_type = DeviceType.PhysicalTarget;
        }
      } else {
        if (this.is_source) {
          final_type = DeviceType.VirtualSource;
        } else {
          final_type = DeviceType.VirtualTarget;
        }
      }

      // CreateNode(NodeType, String),
      let command = {
        "CreateNode": [final_type, name]
      }
      websocket.send_command(command)
    },
  },

  computed: {
    DeviceOrderType() {
      return DeviceOrderType
    },

    button_width() {
      console.log("Called?");
      if (this.has_hidden()) {
        return "70px";
      } else {
        return "50px"
      }
    },
  }

}
</script>

<template>
  <PopupBox ref="popup" @closed="">
    <div class="entry" @click="add_device(false)">
      <span>Add Virtual Channel</span>
    </div>
    <div class="separator"/>
    <div class="entry" @click="add_device(true)">
      <span>Add Physical Channel</span>
    </div>
  </PopupBox>
  <PopupBox ref="hidden" @closed="">
    <div v-for="id of get_hidden()">
      <div class="entry" @click="show_device(id)">
        <span>{{ get_device_by_id(id).description.name }}</span>
      </div>
    </div>
  </PopupBox>

  <div class="mix-list">
    <div class="title">
      <div class="start"></div>
      <div class="text">{{ is_source ? "Sources" : "Targets" }}</div>
      <div class="end">
        <button v-if="has_hidden()" @click="show_hidden">
          <font-awesome-icon :icon="['fas', 'eye-slash']"/>
        </button>
        <span v-if="has_hidden" style="display: inline-block; padding-left: 5px"/>
        <button @click="show_popup">+</button>
      </div>
    </div>
    <div class="device-list">
      <DeviceList v-if="has_pinned()" :is-source="this.is_source"
                  :order-type="DeviceOrderType.Pinned"/>
      <div v-if="has_pinned()" class="split"/>
      <DeviceList :is-source="this.is_source" :order-type="DeviceOrderType.Default"/>
    </div>
  </div>
</template>

<style scoped>
.mix-list {
  display: flex;
  flex-direction: column;
  border: 1px solid #666;
  border-radius: 6px 6px 0 0;
  background-color: #2d3230;
}

.mix-list .title {
  font-weight: bold;
  text-align: center;
  padding-top: 10px;

  display: flex;
  flex-direction: row;
}

.mix-list .title .start {
  width: v-bind(button_width);
}

.mix-list .title .text {
  flex: 1;
}

.mix-list .title .end {
  width: v-bind(button_width);
}

.mix-list .title .end button {
  all: unset;
  height: 20px;
  width: 20px;
  color: #fff;
  border: 1px solid #666666;
  background-color: #353937;
  border-radius: 5px;
}

.mix-list .title .end button:hover {
  cursor: pointer;
}

.mix-list .device-list {
  flex: 1;
  display: flex;
  flex-direction: row;
}

.mix-list .device-list .split {
  flex: 1;
  background-color: #666;
  width: 1px;
  margin-top: 10px;
  margin-bottom: 10px;
}

.separator {
  height: 5px;
  background-color: #3b413f;
}

.title {
  white-space: nowrap;
}

.entry {
  white-space: nowrap;
  padding: 6px 20px 6px 20px;
}

.entry:hover {
  background-color: #49514e;
  cursor: pointer;
}

.entry:not(:last-child) {
  border-bottom: 1px solid #3b413f;
}

</style>
