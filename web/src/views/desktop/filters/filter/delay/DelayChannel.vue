<script>
import NumberInput from "@/views/desktop/filters/layout/inputs/NumberInput.vue";
import DropMenu from "@/views/desktop/filters/layout/inputs/DropMenu.vue";
import {dbToLinear, getFilterConfig, linearToDb, setFilterValue} from "@/app/filters.js";
import Field from "@/views/desktop/filters/layout/Field.vue";

export default {
  name: "DelayChannel",
  components: {Field, DropMenu, NumberInput},
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

    getParam(base) {
      return getFilterConfig(this.filterId).parameters.find(p => p.symbol === `${base}${this.suffix}`);
    },

    setParam(base, value) {
      setFilterValue(this.filterId, `${base}${this.suffix}`, value);
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
  }
}
</script>

<template>
  <Field label="Mode">
    <DropMenu :values="getModes()" :selected="`${activeMode}`"
              @valueClicked="setParam('mode', $event)"/>
  </Field>
  <Field label="Samples" v-if="activeMode === 0">
    <NumberInput :min="getParam('samp').min" :max="getParam('samp').max" :step="1"
                 :value="getParam('samp').value.Int32"
                 @input="setParam('samp', $event)" :allow-empty="false"/>
  </Field>
  <div class="fields-grid" v-if="activeMode === 1">
    <Field label="Meters">
      <NumberInput :min="getParam('m').min" :max="getParam('m').max" :step="1"
                   :value="getParam('m').value.Int32" suffix="m"
                   @input="setParam('m', $event)" :allow-empty="false"/>
    </Field>
    <Field label="Centimeters">
      <NumberInput :min="getParam('cm').min" :max="getParam('cm').max" :step="0.1"
                   :value="getParam('cm').value.Float32" suffix="cm"
                   @input="setParam('cm', $event)" :allow-empty="false"/>
    </Field>
  </div>
  <Field label="Temperature" v-if="activeMode === 1">
    <NumberInput :min="getParam('t').min" :max="getParam('t').max" :step="0.1"
                 :value="getParam('t').value.Float32" suffix="°C"
                 @input="setParam('t', $event)" :allow-empty="false"/>
  </Field>

  <Field label="Time" v-if="activeMode === 2">
    <NumberInput :min="getParam('time').min" :max="getParam('time').max" :step="0.01"
                 :value="getParam('time').value.Float32" suffix="ms"
                 @input="setParam('time', $event)" :allow-empty="false"/>
  </Field>

  <div class="fields-grid">
    <Field label="Dry">
      <NumberInput :min="-80.0" :max="20.0" :step="0.01" suffix="dB"
                   :value="linearToDb(getParam('dry').value.Float32)"
                   @input="setDbParam('dry', $event)" :allow-empty="false"/>
    </Field>

    <Field label="Wet">
      <NumberInput :min="-80.0" :max="20.0" :step="0.01" suffix="dB"
                   :value="linearToDb(getParam('wet').value.Float32)"
                   @input="setDbParam('wet', $event)" :allow-empty="false"/>
    </Field>
  </div>
</template>

<style scoped>
.fields-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  column-gap: 12px;
}
</style>
