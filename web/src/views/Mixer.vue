<script>
import ChannelColumn from "@/views/desktop/channels/ChannelColumn.vue";
import {
  DeviceOrderType,
  DeviceType,
  get_device_by_id,
  get_device_order,
  nameError
} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";
import PopupBox from "@/views/desktop/inputs/PopupBox.vue";
import DeviceList from "@/views/desktop/DeviceList.vue";
import ModalOverlay from "@/views/desktop/components/ModalOverlay.vue";

const INTERNAL_SCALE = 0.8;

export default {
  name: "Mixer",
  components: {ModalOverlay, DeviceList, PopupBox, ChannelColumn},

  data() {
    return {
      textInputValue: "",
      modalIsPhysical: false,
    }
  },

  props: {
    is_source: Boolean,
  },
  methods: {
    nameError,
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
      this.modalIsPhysical = is_physical;

      this.$refs.modal.openModal(this.$refs.textInput, undefined);
    },

    handleOk() {
      if (nameError(this.textInputValue)) return;

      this.do_add_device(this.modalIsPhysical, this.textInputValue);
      this.$refs.modal.closeModal();
    },

    handleCancel() {
      this.$refs.modal.closeModal();
    },

    handleClose() {
      this.textInputValue = "";
    },

    do_add_device(is_physical, name) {
      console.log("DOING IT: " + is_physical + " " + name);

      if (name === undefined || name === "" || name === null) {
        console.log("NO NAME");
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
      websocket.send_command(command).catch(err => {
        alert("Error: " + err);
      });
    }
  },

  computed: {
    DeviceOrderType() {
      return DeviceOrderType
    },

    button_width() {
      if (this.has_hidden()) {
        return "70px";
      } else {
        return "50px"
      }
    },

    nameValidationError() {
      if (this.textInputValue.length === 0) return null;
      return nameError(this.textInputValue);
    }
  }
}
</script>

<template>
  <ModalOverlay ref="modal" id="create" @modal-close="handleClose" @backdrop-click="handleCancel">
    <template #title>Create {{ modalIsPhysical ? "Physical" : "Virtual" }}
      {{ is_source ? "Source" : "Target" }} Device
    </template>

    <div class="modal-content">
      <div style="margin-bottom: 6px;">Device Name:</div>
      <input ref="textInput" maxlength="20" v-model="textInputValue" type="text"
             @keyup.enter="handleOk"/>
      <div v-if="nameValidationError" class="input-error">{{ nameValidationError }}</div>
    </div>

    <template #footer class="modal-footer">
      <button @click="handleCancel" style="margin-right: 10px;">Cancel</button>
      <button @click="handleOk" class="button-default" :disabled="!!nameError(textInputValue)">
        Ok
      </button>
    </template>
  </ModalOverlay>

  <PopupBox ref="popup" @closed="">
    <div class="entry" @click="add_device(false)">
      <span>Add Virtual Device</span>
    </div>
    <div class="separator"/>
    <div class="entry" @click="add_device(true)">
      <span>Add Physical Device</span>
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

.modal-footer {
  background-color: #2d3230;
  text-align: right;
  padding-right: 10px;
  padding-bottom: 10px;
}

.modal-footer button {
  background-color: #353937;
  color: #fff;
  padding: 8px 30px;
  border: 1px solid #2a2e2d;
}

.modal-footer button:hover {
  background-color: #737775;
  cursor: pointer;
}

.modal-footer .button-default {
  background-color: #3b413f
}

.modal-footer .button-default:hover {
  background-color: #737775;
}


.modal-footer button:focus {
  border-color: #4a90d9; /* active border colour */
}

.modal-footer button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.modal-content input[type=text] {
  padding: 5px;
  color: #fff;
  box-sizing: border-box;
  width: 100%;
  border: 1px solid #666;
  outline: none;

  background-color: #2d3230;
}

.modal-content input[type=text]:focus {
  border-color: #4a90d9; /* active border colour */
}

.input-error {
  color: #e06c75;
  font-size: 0.8em;
  margin-top: 4px;
}

</style>
