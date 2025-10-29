<script>
import ColourSettings from '@/views/desktop/channels/ColourSettings.vue'
import ChannelColumnVolume from '@/views/desktop/channels/ChannelColumnVolume.vue'
import {DeviceOrderType, DeviceType, get_device_by_id, is_physical, is_source} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";
import {FontAwesomeIcon} from "@fortawesome/vue-fontawesome";
import PopupBox from "@/views/desktop/inputs/PopupBox.vue";
import MuteTargetSelector from "@/views/desktop/channels/MuteTargetSelector.vue";
import MixAssignment from "@/views/desktop/channels/MixAssignment.vue";
import PhysicalDeviceSelector from "@/views/desktop/channels/DevicePopup.vue";
import DevicePopup from "@/views/desktop/channels/DevicePopup.vue";

export default {
  name: 'ChannelColumn',
  components: {
    DevicePopup,
    PhysicalDeviceSelector,
    MixAssignment,
    MuteTargetSelector, PopupBox, FontAwesomeIcon, ChannelColumnVolume, ColourSettings
  },
  props: {
    type: DeviceType,
    order_group: DeviceOrderType,
    id: String,
  },

  data() {
    return {
      device: null,

      localValue: 50,
      update_locked: false,

      slider_height: 100,

      color_timeout: null,
    }
  },

  mounted() {
    this.$nextTick(() => {
      this.calculateHeight();
      window.addEventListener('resize', this.onResize)
    })
  },

  beforeUnmount() {
    window.removeEventListener('resize', this.onResize)
  },

  methods: {
    onResize: function () {
      this.calculateHeight();
    },

    getDevice: function () {
      return get_device_by_id(this.id);
    },

    getId: function () {
      return this.getDevice().description.id;
    },

    getColour: function () {
      let color = this.getDevice().description.colour;

      return {
        red: color.red,
        green: color.green,
        blue: color.blue
      }
    },

    calculateHeight: function () {
      if (this.$refs.fader_container === undefined) {
        console.log("This shouldn't get called..");
      }

      // Firstly, get the base height (including padding and borders)
      let base_height = this.$refs.fader_container.clientHeight;

      // We have 15px of padding on this element, so remove from top and bottom
      this.slider_height = base_height - 30;
    },

    getVolume: function () {
      if (!is_source(this.type)) {
        return this.getDevice().volume;
      }

      return this.getDevice().volumes.volume.A
    },
    getMixVolume: function () {
      if (!is_source(this.type)) {
        return 0;
      }
      return this.getDevice().volumes.volume.B
    },

    getMuteState: function () {
      if (is_source(this.type)) {
        return this.getDevice().mute_states.mute_state
      } else {
        return this.getDevice().mute_state
      }
    },

    isOutput: function () {
      return !is_source(this.type);
    },


    isMuteA: function () {
      if (this.isOutput()) {
        let state = this.getMuteState();
        return state === "Muted"
      }

      let state = this.getMuteState();
      return state.includes("TargetA");
    },

    isMuteB: function () {
      let state = this.getMuteState();
      return state.includes("TargetB");
    },

    getChannelName: function () {
      return this.getDevice().description.name;
    },
    hasMix: function () {
      return is_source(this.type)
    },
    isLinked: function () {
      if (!this.hasMix()) {
        return false;
      }
      return this.getDevice().volumes.volumes_linked !== null;
    },
    toggleLinked: function () {
      // SetSourceVolumeLinked(Ulid, bool),
      let new_state = !this.isLinked();
      let command = {
        "SetSourceVolumeLinked": [this.getId(), new_state]
      };
      websocket.send_command(command);

    },
    hasBasicMute: function () {
      return true;
    },
    hasComplexMute: function () {
      return is_source(this.type);
    },

    hasMute: function () {
      return this.hasComplexMute() || this.hasBasicMute()
    },

    rgbToHex(r, g, b) {
      return '#' + ((1 << 24) | (r << 16) | (g << 8) | b).toString(16).slice(1)
    },

    hexToRGB(hex) {
      const hexStripped = hex.replace('#', '');
      return {
        red: parseInt(hexStripped.substring(0, 2), 16),
        green: parseInt(hexStripped.substring(2, 4), 16),
        blue: parseInt(hexStripped.substring(4, 6), 16)
      };
    },

    isActiveMix: function (mix) {
      return this.getDevice().mix === mix;
    },

    getMixAColour: function () {
      // If the channel doesn't have a Mix B, check it's assigned mix
      if (!this.hasMix() && this.isActiveMix("B")) {
        return "#E07C24";
      }
      return "#59b1b6";
    },

    volume_changed: function (mix, force, e) {
      if ((!force && !this.update_locked) || force) {
        this.update_locked = true;

        // SetVolume(DeviceType, Ulid, Mix, u8),
        let command = null;
        if (is_source(this.type)) {
          command = {
            "SetSourceVolume": [this.getId(), mix, parseInt(e.target.value)]
          };
        } else {
          command = {
            "SetTargetVolume": [this.getId(), parseInt(e.target.value)]
          };
        }

        websocket.send_command(command).then(() => {
          this.update_locked = false
        });
      }
    },

    isMutedAll(target) {
      let device = this.getDevice();
      return (device.mute_states.mute_targets[target].length === 0);
    },

    mute_click: function (target, e) {
      /*
        AddSourceMuteTarget(Ulid, MuteTarget),
        DelSourceMuteTarget(Ulid, MuteTarget),

        SetTargetMuteState(Ulid, MuteState),
       */

      let mute_target = (target === "A") ? "TargetA" : "TargetB";
      let state = this.getMuteState();

      if (!is_source(this.type)) {
        let new_status = (state === "Unmuted") ? "Muted" : "Unmuted";
        let command = {
          "SetTargetMuteState": [this.getId(), new_status]
        }
        websocket.send_command(command);
      } else {
        let type = (!state.includes(mute_target)) ? "AddSourceMuteTarget" : "DelSourceMuteTarget";
        let command = {
          [type]: [this.getId(), mute_target],
        }
        websocket.send_command(command);
      }
    },

    target_change: function (target, e) {
      // SetTargetMix(Ulid, Mix),
      let command = {
        "SetTargetMix": [this.getId(), target]
      };
      websocket.send_command(command);
    },

    colour_clicked: function (e) {
      const rgb = this.getColour();
      
      const hexColour = this.rgbToHex(rgb.red, rgb.green, rgb.blue);

      this.$refs.colour_picker.value = hexColour;
      this.$refs.colour_picker.click();
    },

    color_dragging: function (e) {
      const hex = e.target.value;
      
      const colorStruct =  this.hexToRGB(hex);

      this.getDevice().description.colour = colorStruct;

      if (this.color_timeout !== null) {
        clearTimeout(this.color_timeout);
      }

      this.color_timeout = setTimeout(() => {
        // SetNodeColour(Ulid, Colour),
        let command = {
          "SetNodeColour": [this.getId(), colorStruct]
        }
        websocket.send_command(command);
        this.color_timeout = null;
      }, 250);
    },

    output_clicked: function (target, e) {
      // Try and locate the button pressed.
      let found = false;
      let element = e.target;
      console.log(element);
      if (element.nodeName.toLowerCase() === "button") {
        console.log(element.firstChild);
        element.firstChild.style.transform = "rotate(-90deg)";
      } else {
        while (!found) {
          if (element.nodeName === "svg" || element.nodeName === "path") {
            element = element.parentNode;
            continue;
          }
          found = true;
        }
        element.style.transform = "rotate(-90deg)";
      }

      //console.log(e);
      this.$refs[target].show(e);
    },

    output_closed: function (e) {
      let target = e + "_icon";
      this.$refs[target].style.transform = "";
    },

    menu_click: function (e) {
      this.$refs.dev_selector.show(e);
    },

    is_physical() {
      return is_physical(this.type);
    },

    is_source() {
      return is_source(this.type);
    }
  },
  computed: {
    colour: function () {
      let colour = this.getColour()
      return `rgb(${colour.red}, ${colour.green}, ${colour.blue})`
    },

    titleBackground: function () {
      // Get the Screen colour..
      let colour = this.getColour()
      let base = `rgba(${colour.red}, ${colour.green}, ${colour.blue}, 0.1)`
      return `linear-gradient(rgba(0,0,0,0), ${base})`
    },

    muteBackground: function () {
      let colour = this.getColour()
      let base = `rgba(${colour.red}, ${colour.green}, ${colour.blue}, 0.3)`
      return `linear-gradient(${base}, rgba(0,0,0,0))`
    },

    topHeight: function () {
      return '7px'
    }
  },
}
</script>

<template>
  <MuteTargetSelector v-if="is_source()" id="mute_a" ref="mute_a" :device_id='id'
                      :type='type'
                      target="TargetA" @closed="output_closed"/>
  <MuteTargetSelector v-if="is_source()" id="mute_b" ref="mute_b" :device_id='id'
                      :type='type' target="TargetB"
                      @closed="output_closed"/>

  <div class="mix">
    <div class="title">
      <div class="start drag-handle">
        <font-awesome-icon :icon="['fas', 'grip-vertical']"/>
      </div>
      <div class="name">{{ getChannelName() }}</div>
      <div class="end">
        <DevicePopup id="select_device" :device_id="id" :order_group='order_group' :type='type' :colour_callback="colour_clicked"/>
      </div>
    </div>
    <input ref="colour_picker" type="color" class="colour_picker" @input="color_dragging"/>
    <div class="top" @click="colour_clicked"></div>
    <div ref="fader_container" class="faders">
      <div class="fader_child">
        <ChannelColumnVolume
          :id="this.id"
          :colour1="getMixAColour()"
          :current-value="getVolume()"
          :height="this.slider_height"
          colour2="#252927"
          @change="event => volume_changed('A', false, event)"
          @input="event => volume_changed('A', true, event)"
        />
        <ChannelColumnVolume
          v-if="hasMix()"
          :id="this.id"
          :current-value="getMixVolume()"
          :height="this.slider_height"
          colour1="#E07C24"
          colour2="#252927"

          @change="event => volume_changed('B', false, event)"
          @input="event => volume_changed('B', true, event)"
        />
      </div>
    </div>
    <div v-if="hasMix()" class="link" @click="toggleLinked">
      <img v-if="isLinked()" alt="Linked" src="/images/submix/linked-white.png"/>
      <img v-else alt="Unlinked" src="/images/submix/unlinked-dimmed.png"/>
    </div>
    <div class="bottom"></div>
    <div v-if="hasMute()" class="mute">
      <div v-if="!hasMix()">
        <MixAssignment :is-mix-a="isActiveMix('A')" @target-change="target_change"/>
      </div>

      <div v-if="hasBasicMute()" :class="{active: isMuteA()}" class="buttons">
        <button @click="event => mute_click('A', event)">
          <span style="width: 16px">
            <font-awesome-icon v-if="isMuteA()" :icon="['fas', 'volume-xmark']"/>
            <font-awesome-icon v-else :icon="['fas', 'volume-high']"/>
          </span>
          <span v-if="isOutput()">Mute Channel</span>
          <span v-else-if="isMutedAll('TargetA')">Mute to All</span>
          <span v-else>Mute to...</span>
        </button>
        <button v-if="!isOutput()" @click="e => output_clicked('mute_a', e)">
          <span ref="mute_a_icon" class="rotate">
            <font-awesome-icon :icon="['fas', 'angle-down']"/>
          </span>
        </button>
      </div>
      <div v-if="hasComplexMute()" :class="{active: isMuteB()}" class="buttons">
        <button @click="event => mute_click('B', event)">
          <span style="width: 16px">
            <font-awesome-icon v-if="isMuteB()" :icon="['fas', 'volume-xmark']"/>
            <font-awesome-icon v-else :icon="['fas', 'volume-high']"/>
          </span>
          <span v-if="isMutedAll('TargetB')">Mute to All</span>
          <span v-else>Mute to...</span>
        </button>
        <button @click="e => output_clicked('mute_b', e)">
          <span ref="mute_b_icon" class="rotate">
            <font-awesome-icon :icon="['fas', 'angle-down']"/>
          </span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mix {
  height: 100%;
  min-width: 150px;
  background-color: #353937;
  border: 1px solid #666666;
  border-radius: 3px 3px 0 0;

  display: flex;
  flex-direction: column;
}

.title {
  display: flex;
  flex-direction: row;
  padding: 8px;
  text-align: center;
  font-weight: bold;
  background: v-bind(titleBackground);
}

.title .start {
  width: 20px;
}

.title .start:hover {
  cursor: move;
}

.title .name {
  flex: 1;
}

.title .end {
  width: 20px;
}

.title .end button {
  all: unset;
  border: 0;
  background-color: transparent;
  color: #fff;
  padding: 0;
  margin: 0;
}

.title .end button:hover {
  cursor: pointer;
}

.colour_picker {
  position: absolute;
  visibility: hidden;
}

.top {
  background-color: v-bind(colour);
  height: v-bind(topHeight);
  transform-origin: 0 0;
  transition: transform 0.2s ease, filter 0.2s ease;
  will-change: transform, filter;
}

.top:hover {
  cursor: pointer;
  filter: brightness(1.25);
  transform: scaleY(1.5);
}

.faders {
  position: relative;
  padding: 15px;
  flex: 1;
}

.fader_child {
  position: absolute;

  left: 50%;
  transform: translate(-50%, 0);

  display: flex;
  flex-direction: row;
  justify-content: center;

  width: 20px;
  gap: 35px;
}

.link {
  text-align: center;
  height: 30px;
}

.link img {
  height: 20px;
}

.assignment {
  text-align: center;
  height: 30px;
}

.bottom {
  background-color: v-bind(colour);
  height: 5px;
}

.mute {
  background: v-bind(muteBackground);

  display: flex;
  flex-direction: column;
  gap: 5px;
}

.mute.small {
  height: 30px;
}

.mute > div {
  display: flex;
  flex-direction: row;

  font-size: 1em;
  flex-grow: 1;
}

.buttons button {
  all: unset;
}

.buttons button:first-child {
  flex: 1;
}

.mute .buttons div,
.mute .buttons button {
  background-color: rgba(80, 80, 80, 0.6);
  overflow: hidden;

  border: 1px solid #666;
  border-left: 0;
  border-right: 0;

  display: flex;
  align-items: center;
}

.mute .active {
  transition: all 0.4s ease;
  background-color: rgba(255, 0, 0, 0.6);
}

.mute .buttons button span {
  display: inline-block;
  margin-left: 4px;
  margin-right: 5px;
}

.mute .buttons button span img {
  width: 24px;
  fill: #fff;
}

.mute .buttons button span.label {
  width: 90px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
}

.mute .buttons button span.rotate {
  padding: 0;
  margin: 0;
  transition: transform 0.2s ease;
}

.mute .buttons div:first-child,
.mute .buttons button:first-child:not(:last-child) {
  border-right: 1px solid #666;
}

.mute .buttons div:last-child,
.mute .buttons button:last-child {
  padding: 4px;
}

.mute :first-child > div,
.mute :first-child > button {
  border-top: 0;
}

.mute :last-child > div,
.mute :last-child > button {
  border-bottom: 0;
}
</style>
