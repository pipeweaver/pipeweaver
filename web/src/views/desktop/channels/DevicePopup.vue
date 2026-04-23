<script>
// A lot of this is copied for MuteTargetSelector, mostly because I'm lazy :D

import PopupBox from "@/views/desktop/inputs/PopupBox.vue";
import {
  DeviceOrderType,
  DeviceType,
  get_device_by_id,
  get_device_type,
  getSourcePhysicalDevices,
  getTargetPhysicalDevices,
  is_physical,
  is_source, isValidName
} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";
import ModalOverlay from "@/views/desktop/components/ModalOverlay.vue";

export default {
  name: "DevicePopup",
  computed: {
    DeviceOrderType() {
      return DeviceOrderType
    },

    nameValidationError() {
      if (this.textInputValue.length === 0) return null;
      return this.isValidName(this.textInputValue);
    }
  },
  components: {ModalOverlay, PopupBox},

  props: {
    type: {type: String, required: true},
    device_id: {type: String, required: true},
    order_group: {type: String, required: true},
    id: {type: String, required: true},
    colour_callback: {type: Function, required: true}
  },

  data() {
    return {
      textInputValue: "",
    }
  },

  methods: {
    isValidName,

    show(e) {
      this.$refs.popup.showDialog(e, this.id)
    },

    getDevice() {
      return get_device_by_id(this.device_id);
      //return get_devices(this.type)[this.index];
    },

    getId: function () {
      return this.getDevice().description.id;
    },

    getName: function () {
      return this.getDevice().description.name;
    },

    getType: function () {
      return this.isPhysicalNode() ? "Physical" : "Virtual";
    },

    isPhysicalNode: function () {
      return is_physical(this.type);
    },

    isTargetNode: function () {
      return get_device_type(this.getId()) === DeviceType.VirtualTarget;
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
          node_id = device.node_id;
          name = device.name;
          description = device.description;

          // Flag this as already found, so we don't repeat later
          mapped_node_ids.push(device.node_id);
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
        if (!mapped_node_ids.includes(device.node_id)) {
          list.push({
            node_id: device.node_id,
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

    menuClick: function (e) {
      this.$refs.popup.showDialog(e, this.id);
    },

    onClick: function (device) {
      // AttachPhysicalNode(Ulid, u32),
      // RemovePhysicalNode(Ulid, usize),

      // Ok, work out what we need to do here
      if (this.isConfigDevice(device)) {
        let command = {
          "RemovePhysicalNode": [this.getId(), device.config_id]
        }
        websocket.send_command(command);
        return;
      }

      let command = {
        "AttachPhysicalNode": [this.getId(), device.node_id]
      }
      websocket.send_command(command);
    },

    onRenameClick: function () {
      this.$refs.popup.hideDialog();
      this.$refs.modal.openModal(this.$refs.textInput, undefined);
    },

    handleRenameOk: function () {
      if (this.nameValidationError) return;

      this.doRename(this.textInputValue);
      this.$refs.modal.closeModal();
    },

    handleRenameCancel: function () {
      this.$refs.modal.closeModal();
    },

    handleRenameClose: function () {
      this.textInputValue = "";
    },

    doRename(name) {
      if (name !== null) {
        let command = {
          "RenameNode": [this.getId(), name]
        }
        websocket.send_command(command).catch(err => {
          alert("Error: " + err);
        });
        this.$refs.popup.hideDialog();
      }
    },

    onRemoveClick: function () {
      this.$refs.popup.hideDialog();
      this.$refs.remove_modal.openModal(this.$refs.okButton, undefined);
    },

    handleRemoveOk: function () {
      this.doRemove(this.textInputValue);
      this.$refs.remove_modal.closeModal();
    },

    handleRemoveCancel: function () {
      this.$refs.remove_modal.closeModal();
    },

    doRemove() {
      let command = {
        "RemoveNode": this.getId()
      }
      websocket.send_command(command);
    },

    onPinClicked(toggle, e) {
      //    SetOrderGroup(Ulid, OrderGroup),
      let command = {
        "SetOrderGroup": [this.getId(), (toggle) ? DeviceOrderType.Pinned : DeviceOrderType.Default]
      }
      websocket.send_command(command);
      this.$refs.popup.hideDialog();
    },

    onHideClicked(e) {
      let command = {
        "SetOrderGroup": [this.getId(), DeviceOrderType.Hidden]
      }
      websocket.send_command(command);
      this.$refs.popup.hideDialog();
    },

    onColourClicked(e) {
      this.colour_callback(e);
      this.$refs.popup.hideDialog();
    }
  },
}
</script>

<template>
  <ModalOverlay ref="modal" id="rename" @modal-close="handleRenameClose"
                @backdrop-click="handleRenameCancel">
    <template #title>Rename {{ getName() }}</template>

    <div class="modal-content">
      <div style="margin-bottom: 6px;">New Name:</div>
      <input ref="textInput" v-model="textInputValue" maxlength="20" type="text"
             @keyup.enter="handleRenameOk"/>
      <div v-if="nameValidationError" class="input-error">{{ nameValidationError }}</div>
    </div>

    <template #footer class="modal-footer">
      <button @click="handleRenameCancel" style="margin-right: 10px;">Cancel</button>
      <button @click="handleRenameOk" class="default" :disabled="!!isValidName(textInputValue)">
        Ok
      </button>
    </template>
  </ModalOverlay>

  <ModalOverlay ref="remove_modal" id="remove" @backdrop-click="handleRemoveCancel">
    <template #title>Remove {{ getName() }}</template>

    <div class="modal-content">
      <div>Are you sure you want to remove this device?</div>
    </div>

    <template #footer class="modal-footer">
      <button @click="handleRemoveCancel" style="margin-right: 10px;">No</button>
      <button ref="okButton" @click="handleRemoveOk" class="button-default">Yes</button>
    </template>
  </ModalOverlay>

  <button @click="menuClick">
    <font-awesome-icon :icon="['fas', 'bars']"/>
  </button>

  <PopupBox ref="popup" @closed="">
    <div class="popup-title">
      <span>{{ getName() }} - ({{ getType() }})</span>
    </div>
    <div class="title-separator"/>
    <div class="entry" @click="onColourClicked">
      <span class="color_icon"></span>
      <span>Change Colour</span>
    </div>
    <div class="separator"/>
    <div v-if="order_group !== DeviceOrderType.Pinned" class="entry"
         @click="e => onPinClicked(true, e)">
      <span class="selected"></span>
      <span>Pin Channel</span>
    </div>
    <div v-if="order_group === DeviceOrderType.Pinned" class="entry"
         @click="e => onPinClicked(false, e)">
      <span class="selected"></span>
      <span>Unpin Channel</span>
    </div>
    <div class="entry" @click="e => onHideClicked(false, e)">
      <span class="selected"></span>
      <span>Hide Channel</span>
    </div>
    <div v-if="isPhysicalNode() || isTargetNode()" class="separator"/>
    <div v-if="!isPhysicalNode() && isTargetNode()" class="entry-base">
      Duplicate Output To:
    </div>
    <div v-for="device of getDevices()" v-if="isPhysicalNode() || isTargetNode()"
         :class="{error: !isDevicePresent(device)}"
         class="entry"
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
    <div class="entry" @click="onRenameClick">
      <span class="selected"></span>
      <span>Rename Channel</span>
    </div>
    <div class="entry" @click="onRemoveClick">
      <span class="selected"></span>
      <span>Remove Channel</span>
    </div>
  </PopupBox>
</template>

<style scoped>
button {
  all: unset;
}

button:hover {
  cursor: pointer;
}

.separator {
  height: 5px;
  background-color: #3b413f;
}

.title-separator {
  height: 2px;
  background-color: v-bind('getDevice().description.colour ? `rgb(${getDevice().description.colour.red}, ${getDevice().description.colour.green}, ${getDevice().description.colour.blue})` : "#000000"');
  border-bottom: 3px solid #3b413f;
}

.selected {
  display: inline-block;
  width: 20px;
  padding: 0;
}

.color_icon {
  display: inline-block;
  width: 1em;
  height: 1em;
  margin-right: calc(20px - 1em);
  border-radius: 50%;
  background-color: v-bind('getDevice().description.colour ? `rgb(${getDevice().description.colour.red}, ${getDevice().description.colour.green}, ${getDevice().description.colour.blue})` : "#000000"');
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

.popup-title {
  white-space: nowrap;
  padding: 6px 60px;
  font-weight: bold;
}

.entry-base {
  font-weight: normal;
  font-style: italic;
  white-space: nowrap;
  padding: 6px 6px 6px 10px;
  text-align: left;
  display: flex;
  align-items: center;
  border-bottom: 1px solid #3b413f;
}

.entry {
  white-space: nowrap;
  padding: 6px 25px 6px 6px;
  text-align: left;
  display: flex;
  align-items: center;
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

span {
  text-align: left;
}

.modal-content {
  text-align: left;
  font-weight: normal;
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

.modal-footer button:focus {
  border-color: #4a90d9;
}

.modal-footer .default {
  background-color: #3b413f
}

.modal-footer .default:hover {
  background-color: #737775;
}

.modal-footer button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.modal-content input[type=text] {
  text-align: left;

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
