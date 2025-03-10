<script>
import ColourSettings from '@/components/channels/ColourSettings.vue'
import MuteState from '@/components/channels/MuteState.vue'
import ChannelColumnVolume from '@/components/channels/ChannelColumnVolume.vue'
import {DeviceType, get_devices, is_source} from "@/pipecast/util.js";
import {websocket} from "@/pipecast/sockets.js";

export default {
  name: 'ChannelColumn',
  components: {ChannelColumnVolume, ColourSettings, MuteState},
  props: {
    type: DeviceType,
    index: Number,
    // channel: { type: String, required: false },
    // title: { type: String, required: true }
  },

  data() {
    return {
      localValue: 50,
      window_size: window.innerHeight,

      update_locked: false,
    }
  },

  mounted() {
    this.$nextTick(() => {
      window.addEventListener('resize', this.onResize)
    })
  },

  beforeUnmount() {
    window.removeEventListener('resize', this.onResize)
  },

  methods: {
    onResize: function () {
      this.window_size = window.innerHeight
    },

    getDevice: function () {
      return get_devices(this.type)[this.index];
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

    isMutePressActive: function () {
      return this.getMute().mute_state === 'Pressed'
    },
    isMuteHoldActive: function () {
      return this.getMute().mute_state === 'Held'
    },
    getMutePressTargets: function () {
      return this.getMute().mute_actions.Press
    },
    getMuteHoldTargets: function () {
      return this.getMute().mute_actions.Hold
    },

    calculateHeight: function () {
      // We'll start with a base 'full' slider height
      let size = Math.max(this.window_size - 400, 220)

      // Remove 30px for either the Link item on Inputs, or the Mix Option on Outputs
      size -= 30

      // // If sub-mixes are available, cut 30px for the link icon
      // if (this.hasMix()) {
      //   size -= 30
      // }

      // Cut 30 for the first button (if applicable)
      if (this.hasBasicMute()) {
        size -= 30
      }

      // Cut 30 for the Second Button (if applicable)
      if (this.hasComplexMute()) {
        size -= 30
      }

      // If we're showing two buttons, cut 5 more for the 'gap' between them
      if (this.hasBasicMute() && this.hasComplexMute()) {
        size -= 5
      }

      // Done :)
      return size
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
    getMute: function () {
      //return store.getActiveDevice().config.device.channels.configs[this.getChannelName()]
    },

    getChannelName: function () {
      return this.getDevice().description.name;
    },
    hasMix: function () {
      return is_source(this.type)
    },
    isLinked: function () {
      return false;
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

    isActiveMix: function (mix) {
      return this.getDevice().mix === mix;
    },

    getMixAColour: function () {
      // If the channel doesn't have a Mix B, check it's assigned mix
      if (!this.hasMix() && this.isActiveMix("A")) {
        return "#E07C24";
      }
      return "#59b1b6";
    },

    volume_changed: function (mix, force, e) {
      if ((!force && !this.update_locked) || force) {
        this.update_locked = true;

        // SetVolume(Ulid, Mix, u8),
        let command = {
          "SetVolume": [this.getId(), mix, parseInt(e.target.value)]
        }
        console.log(command);

        websocket.send_command(command).then(() => {
          this.update_locked = false
        });
      }
    },

    colour_clicked: function (e) {
      console.log("Colour Clicked: {}", e);
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
  }
}
</script>

<template>
  <div class="mix">
    <div class="title">{{ getChannelName() }}</div>
    <div class="top" @click="colour_clicked"></div>
    <div class="faders">
      <ChannelColumnVolume
        :colour1="getMixAColour()"
        :current-value="getVolume()"
        :height="calculateHeight()"
        colour2="#252927"
        @change="event => volume_changed('A', false, event)"
        @input="event => volume_changed('A', true, event)"
      />
      <ChannelColumnVolume
        v-if="hasMix()"
        :current-value="getMixVolume()"
        :height="calculateHeight()"
        colour1="#E07C24"
        colour2="#252927"

        @change="event => volume_changed('B', false, event)"
        @input="event => volume_changed('B', true, event)"
      />
    </div>
    <div v-if="hasMix()" class="link">
      <img v-if="isLinked()" alt="Linked" src="/images/submix/linked-white.png"/>
      <img v-else alt="Unlinked" src="/images/submix/unlinked-dimmed.png"/>
    </div>
    <div v-if="!hasMix()" class="assignment">
      A:<input :checked="isActiveMix('A')" :name="`mix-${getId()}`" type="radio">
      B:<input :checked="isActiveMix('B')" :name="`mix-${getId()}`" type="radio">
    </div>
    <div class="bottom"></div>
    <div v-if="hasMute()" :class="[!hasComplexMute() ? 'small' : '']" class="mute">
      <div v-if="hasBasicMute()" class="buttons">
        <button>
          <span>
            <img alt="Press" src="/images/hold.svg"/>
          </span>
          <span>Mute to All</span>
        </button>
        <button>
          <font-awesome-icon :icon="['fas', 'angle-down']"/>
        </button>
      </div>
      <div v-if="hasComplexMute()" class="buttons">
        <button>
          <span>
            <img alt="Press" src="/images/press.svg"/>
          </span>
          <span class="label">Mute to Headphones</span>
        </button>
        <button>
          <font-awesome-icon :icon="['fas', 'angle-down']"/>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mix {
  min-width: 150px;
  background-color: #353937;
  border: 1px solid #666666;
  border-radius: 5px;
}

.title {
  padding: 8px;
  text-align: center;
  font-weight: bold;
  background: v-bind(titleBackground);
}

.top {
  background-color: v-bind(colour);
  height: v-bind(topHeight);
}

.faders {
  padding: 15px;
  display: flex;
  flex-direction: row;
  justify-content: center;
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
  height: 65px;
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
  background-color: rgba(80, 80, 80, 0.8);
  overflow: hidden;

  border: 1px solid #666;
  border-left: 0;
  border-right: 0;

  display: flex;
  align-items: center;
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

.mute .buttons div:first-child,
.mute .buttons button:first-child {
  border-right: 1px solid #666;
}

.mute .buttons div:last-child,
.mute .buttons button:last-child {
  padding: 4px;
  border-left: 1px solid #555;
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
