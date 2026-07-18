<script>
import NumberInput from "@/views/desktop/filters/layout/inputs/NumberInput.vue";
import Toggle from "@/views/desktop/filters/layout/inputs/Toggle.vue";
import DropMenu from "@/views/desktop/filters/layout/inputs/DropMenu.vue";
import FlowLayout from "@/views/desktop/filters/layout/FlowLayout.vue";
import FlowItem from "@/views/desktop/filters/layout/FlowItem.vue";
import Field from "@/views/desktop/filters/layout/Field.vue";
import {dbToLinear, getFilterConfig, linearToDb, setFilterValue} from "@/app/filters.js";

export default {
  name: "CrusherFilter",
  components: {FlowItem, Field, FlowLayout, DropMenu, NumberInput, Toggle},
  props: {
    filterId: {type: String, required: true},
    filterType: {type: String, required: true}
  },

  methods: {
    getParam(symbol) {
      return getFilterConfig(this.filterId).parameters.find(p => p.symbol === symbol);
    },

    setParam(symbol, value) {
      setFilterValue(this.filterId, symbol, value);
    },

    setDbParam(symbol, value) {
      this.setParam(symbol, dbToLinear(value));
    },

    getDb(symbol) {
      return linearToDb(this.getParam(symbol).value.Float32);
    },

    getPercent(symbol) {
      return this.getParam(symbol).value.Float32 * 100;
    },

    setPercent(symbol, value) {
      this.setParam(symbol, value / 100);
    },

    boolOptions() {
      return [{value: 'false', text: 'Off'}, {value: 'true', text: 'On'}];
    },

    modeOptions() {
      return [
        {value: '0', text: 'Linear'},
        {value: '1', text: 'Logarithmic'},
      ];
    },
  }
}
</script>

<template>
  <div style="padding: 10px">
    <FlowLayout>
      <FlowItem width="220px">
        <div class="title">Shape</div>

        <Field label="Bit Reduction">
          <NumberInput :min="getParam('bits').min" :max="getParam('bits').max" :step="0.1"
                       :value="getParam('bits').value.Float32"
                       @input="setParam('bits', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Mode">
          <DropMenu :values="modeOptions()" :selected="`${getParam('mode').value.Int32}`"
                    @valueClicked="setParam('mode', $event)"/>
        </Field>
        <Field label="DC Offset">
          <NumberInput :min="-12" :max="12" :step="0.1" suffix="dB"
                       :value="getDb('dc')" @input="setDbParam('dc', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Anti-aliasing">
          <NumberInput :min="0" :max="100" :step="1" suffix="%"
                       :value="getPercent('anti_aliasing')"
                       @input="setPercent('anti_aliasing', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Mix">
          <NumberInput :min="0" :max="100" :step="1" suffix="%"
                       :value="getPercent('morph')"
                       @input="setPercent('morph', $event)" :allow-empty="false"/>
        </Field>
      </FlowItem>

      <FlowItem width="180px">
        <div class="title">Sample Rate</div>

        <Field label="Reduction">
          <NumberInput :min="getParam('samples').min" :max="getParam('samples').max" :step="1"
                       :value="getParam('samples').value.Int32"
                       @input="setParam('samples', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Low Frequency Oscillator" row>
          <Toggle :value="getParam('lfo').value.Bool" @input="setParam('lfo', $event)"/>
        </Field>
        <Field label="Range" :disabled="!getParam('lfo').value.Bool">
          <NumberInput :min="getParam('lforange').min" :max="getParam('lforange').max" :step="1"
                       :value="getParam('lforange').value.Int32"
                       @input="setParam('lforange', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Rate" :disabled="!getParam('lfo').value.Bool">
          <NumberInput :min="getParam('lforate').min" :max="getParam('lforate').max" :step="0.1"
                       suffix="Hz"
                       :value="getParam('lforate').value.Float32"
                       @input="setParam('lforate', $event)" :allow-empty="false"/>
        </Field>
      </FlowItem>
    </FlowLayout>
  </div>
</template>

<style scoped>
.title {
  font-size: 1.1em;
  font-weight: 600;
  margin-bottom: 0.6em;
}


</style>
