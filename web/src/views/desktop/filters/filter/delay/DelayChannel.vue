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

  data() {
    return {
      mode: '0',
    }
  },

  methods: {
    linearToDb,

    getFilterConfig() {
      return store.getAudio().filter_config[this.filterId];
    },

    get_modes() {
      return [
        {value: '0', text: 'Samples'},
        {value: '1', text: 'Distance'},
        {value: '2', text: 'Time'},
      ]
    },
    get_active_mode() {
      let key = this.channel === 'l' ? 'mode_l' : 'mode_r';
      console.log(this.getFilterConfig().parameters);
      return this.getFilterConfig().parameters.find(p => p.symbol === key);
    },
    set_active_mode(value) {
      this.setParameterValue(this.channel === 'l' ? 'mode_l' : 'mode_r', value);
    },

    // Value fetching
    get_sample_config() {
      let key = this.channel === 'l' ? 'samp_l' : 'samp_r';
      return this.getFilterConfig().parameters.find(p => p.symbol === key);
    },
    set_sample_value(value) {
      this.setParameterValue(this.channel === 'l' ? 'samp_l' : 'samp_r', value);
    },

    get_meter_config() {
      let key = this.channel === 'l' ? 'm_l' : 'm_r';
      return this.getFilterConfig().parameters.find(p => p.symbol === key);
    },
    set_meter_value(value) {
      this.setParameterValue(this.channel === 'l' ? 'm_l' : 'm_r', value);
    },

    get_centimeter_config() {
      let key = this.channel === 'l' ? 'cm_l' : 'cm_r';
      return this.getFilterConfig().parameters.find(p => p.symbol === key);
    },
    set_centimeter_value(value) {
      this.setParameterValue(this.channel === 'l' ? 'cm_l' : 'cm_r', value);
    },

    get_temperature_config() {
      let key = this.channel === 'l' ? 't_l' : 't_r';
      return this.getFilterConfig().parameters.find(p => p.symbol === key);
    },
    set_temperature_value(value) {
      this.setParameterValue(this.channel === 'l' ? 't_l' : 't_r', value);
    },

    get_time_config() {
      let key = this.channel === 'l' ? 'time_l' : 'time_r';
      return this.getFilterConfig().parameters.find(p => p.symbol === key);
    },
    set_time_value(value) {
      this.setParameterValue(this.channel === 'l' ? 'time_l' : 'time_r', value);
    },

    get_dry() {
      let key = this.channel === 'l' ? 'dry_l' : 'dry_r';
      return this.getFilterConfig().parameters.find(p => p.symbol === key);
    },
    set_dry_value(value) {
      let parsed = dbToLinear(value);
      this.setParameterValue(this.channel === 'l' ? 'dry_l' : 'dry_r', parsed);
    },

    get_wet() {
      let key = this.channel === 'l' ? 'wet_l' : 'wet_r';
      return this.getFilterConfig().parameters.find(p => p.symbol === key);
    },
    set_wet_value(value) {
      let parsed = dbToLinear(value);
      this.setParameterValue(this.channel === 'l' ? 'wet_l' : 'wet_r', parsed);
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
    <div class="title">
      <div v-if="channel === 'l'">Left</div>
      <div v-else>Right</div>
    </div>
    <div>
      <div>Mode</div>
      <DropMenu :values="get_modes()" :selected="`${get_active_mode().value.Int32}`"
                @valueClicked="set_active_mode"/>
    </div>
    <div id="samples" v-if="get_active_mode().value.Int32 === 0">
      <div>Samples</div>
      <div>
        <NumberInput :min="get_sample_config().min" :max="get_sample_config().max" :step="1"
                     :value="get_sample_config().value.Int32" @input="set_sample_value"
                     :allow-empty="false"/>
      </div>
    </div>
    <div id="distance" v-if="get_active_mode().value.Int32 === 1">
      <div class="split">
        <div>
          <div>Meters</div>
          <div>
            <NumberInput :min="get_meter_config().min" :max="get_meter_config().max" :step="1"
                         :value="get_meter_config().value.Int32" :suffix="'m'"
                         @input="set_meter_value" :allow-empty="false"/>
          </div>
        </div>
        <div>
          <div>Centimeters</div>
          <div>
            <NumberInput :min="get_centimeter_config().min" :max="get_centimeter_config().max"
                         :value="get_centimeter_config().value.Float32" :step="0.1" :suffix="'cm'"
                         @input="set_centimeter_value" :allow-empty="false"/>
          </div>
        </div>
      </div>
      <div>
        <div>Temperature</div>
        <div>
          <NumberInput :min="get_temperature_config().min" :max="get_temperature_config().max"
                       :value="get_temperature_config().value.Float32" :step="0.1" :suffix="'°C'"
                       @input="set_temperature_value" :allow-empty="false"/>
        </div>
      </div>
    </div>
    <div id="time" v-if="get_active_mode().value.Int32 === 2">
      <div>Time</div>
      <div>
        <NumberInput :min="get_time_config().min" :max="get_time_config().max"
                     :value="get_time_config().value.Float32" :step="1" :suffix="'ms'"
                     @input="set_time_value" :allow-empty="false"/>
      </div>
    </div>
    <div class="split">
      <div>
        <div>Dry</div>
        <div>
          <NumberInput :min="-80.0" :max="20.0" :value="linearToDb(get_dry().value.Float32)"
                       :step="0.01" :suffix="'dB'" @input="set_dry_value" :allow-empty="false"/>
        </div>
      </div>
      <div>
        <div>Wet</div>
        <div>
          <NumberInput :min="-80.0" :max="20.0" :value="linearToDb(get_wet().value.Float32)"
                       :step="0.01" :suffix="'dB'" @input="set_wet_value" :allow-empty="false"/>
        </div>
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
