<script>
import NumberInput from "@/views/desktop/filters/layout/inputs/NumberInput.vue";
import DropMenu from "@/views/desktop/filters/layout/inputs/DropMenu.vue";
import {websocket} from "@/app/sockets.js";
import {store} from "@/app/store.js";
import {dbToLinear, linearToDb} from "@/app/audio.js";

export default {
  name: "DelayChannel",
  components: {DropMenu, NumberInput},
  props: {
    channel: {type: String, required: true},
    filterId: {type: String, required: true},
  },

  computed: {
    suffix() {
      return this.channel === 'l' ? '_l' : '_r';
    },
    activeMode() {
      return this.getParam('mode').value.Int32;
    },
  },

  methods: {
    linearToDb,

    getFilterConfig() {
      return store.getAudio().filter_config[this.filterId];
    },

    getParam(base) {
      return this.getFilterConfig().parameters.find(p => p.symbol === `${base}${this.suffix}`);
    },

    setParam(base, value) {
      this.setParameterValue(`${base}${this.suffix}`, value);
    },

    setDbParam(base, value) {
      this.setParam(base, dbToLinear(value));
    },

    getModes() {
      return [
        {value: '0', text: 'Samples'},
        {value: '1', text: 'Distance'},
        {value: '2', text: 'Time'},
      ];
    },

    setParameterValue(paramName, value) {
      const param = this.getFilterConfig().parameters.find(p => p.symbol === paramName);
      const id = parseInt(param.id);

      let send_value;
      if (typeof value === 'boolean' || value === 'true' || value === 'false') {
        send_value = {"Bool": value === true || value === 'true'};
      } else if ('Int32' in param.value) {
        send_value = {"Int32": parseInt(value)};
      } else if ('Float32' in param.value) {
        send_value = {"Float32": parseFloat(value)};
      } else {
        send_value = {"Float32": parseFloat(value)};
      }

      websocket.send_command({"SetFilterValue": [this.filterId, id, send_value]});
    },
  }
}
</script>

<template>
  <div class="top">
    <div class="title">{{ channel === 'l' ? 'Left' : 'Right' }}</div>

    <div>
      <div>Mode</div>
      <DropMenu :values="getModes()" :selected="`${activeMode}`"
                @valueClicked="setParam('mode', $event)"/>
    </div>

    <div id="samples" v-if="activeMode === 0">
      <div>Samples</div>
      <NumberInput :min="getParam('samp').min" :max="getParam('samp').max" :step="1"
                   :value="getParam('samp').value.Int32"
                   @input="setParam('samp', $event)" :allow-empty="false"/>
    </div>

    <div id="distance" v-if="activeMode === 1">
      <div class="split">
        <div>
          <div>Meters</div>
          <NumberInput :min="getParam('m').min" :max="getParam('m').max" :step="1"
                       :value="getParam('m').value.Int32" suffix="m"
                       @input="setParam('m', $event)" :allow-empty="false"/>
        </div>
        <div>
          <div>Centimeters</div>
          <NumberInput :min="getParam('cm').min" :max="getParam('cm').max" :step="0.1"
                       :value="getParam('cm').value.Float32" suffix="cm"
                       @input="setParam('cm', $event)" :allow-empty="false"/>
        </div>
      </div>
      <div>
        <div>Temperature</div>
        <NumberInput :min="getParam('t').min" :max="getParam('t').max" :step="0.1"
                     :value="getParam('t').value.Float32" suffix="°C"
                     @input="setParam('t', $event)" :allow-empty="false"/>
      </div>
    </div>

    <div id="time" v-if="activeMode === 2">
      <div>Time</div>
      <NumberInput :min="getParam('time').min" :max="getParam('time').max" :step="0.01"
                   :value="getParam('time').value.Float32" suffix="ms"
                   @input="setParam('time', $event)" :allow-empty="false"/>
    </div>

    <div class="split">
      <div>
        <div>Dry</div>
        <NumberInput :min="-80.0" :max="20.0" :step="0.01" suffix="dB"
                     :value="linearToDb(getParam('dry').value.Float32)"
                     @input="setDbParam('dry', $event)" :allow-empty="false"/>
      </div>
      <div>
        <div>Wet</div>
        <NumberInput :min="-80.0" :max="20.0" :step="0.01" suffix="dB"
                     :value="linearToDb(getParam('wet').value.Float32)"
                     @input="setDbParam('wet', $event)" :allow-empty="false"/>
      </div>
    </div>
  </div>
</template>

<style scoped>
.title {
  font-size: 1.2em;
  margin-bottom: 0.5em;
}

.split {
  display: flex;
  flex-direction: row;
  gap: 0.5em;
}

.split > div {
  flex: 1;
  min-width: 0;
}

.split input {
  width: 100%;
  box-sizing: border-box;
}
</style>
