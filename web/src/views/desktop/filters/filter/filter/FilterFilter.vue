<script>
import NumberInput from "@/views/desktop/filters/layout/inputs/NumberInput.vue";
import DropMenu from "@/views/desktop/filters/layout/inputs/DropMenu.vue";
import FlowLayout from "@/views/desktop/filters/layout/FlowLayout.vue";
import FlowItem from "@/views/desktop/filters/layout/FlowItem.vue";
import Field from "@/views/desktop/filters/layout/Field.vue";
import {dbToLinear, getFilterConfig, linearToDb, setFilterValue} from "@/app/filters.js";

export default {
  name: "FilterFilter",
  components: {FlowItem, Field, FlowLayout, DropMenu, NumberInput},
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

    typeOptions() {
      return [
        {value: '0', text: 'Low-pass'},
        {value: '1', text: 'High-pass'},
        {value: '2', text: 'Low-shelf'},
        {value: '3', text: 'High-shelf'},
        {value: '4', text: 'Bell'},
        {value: '5', text: 'Band-pass'},
        {value: '6', text: 'Notch'},
        {value: '7', text: 'Resonance'},
        {value: '8', text: 'Ladder-pass'},
        {value: '9', text: 'Ladder-rejection'},
        {value: '10', text: 'All-pass'},
      ];
    },

    filterModeOptions() {
      return [
        {value: '0', text: 'RLC (BT)'},
        {value: '1', text: 'RLC (MT)'},
        {value: '2', text: 'BWC (BT)'},
        {value: '3', text: 'BWC (MT)'},
        {value: '4', text: 'LRX (BT)'},
        {value: '5', text: 'LRX (MT)'},
        {value: '6', text: 'APO (DR)'},
      ];
    },

    equalModeOptions() {
      return [
        {value: '0', text: 'IIR'},
        {value: '1', text: 'FIR'},
        {value: '2', text: 'FFT'},
        {value: '3', text: 'SPM'},
      ];
    },

    decrampOptions() {
      return [
        {value: '0', text: 'Off'},
        {value: '1', text: 'x2'},
        {value: '2', text: 'x3'},
        {value: '3', text: 'x4'},
        {value: '4', text: 'x6'},
        {value: '5', text: 'x8'},
      ];
    },

    slopeOptions() {
      return [
        {value: '0', text: 'x1'},
        {value: '1', text: 'x2'},
        {value: '2', text: 'x3'},
        {value: '3', text: 'x4'},
        {value: '4', text: 'x6'},
        {value: '5', text: 'x8'},
        {value: '6', text: 'x12'},
        {value: '7', text: 'x16'},
      ];
    },
  }
}
</script>

<template>
  <div style="padding: 10px">
    <FlowLayout>
      <FlowItem width="200px">
        <div class="title">Controls</div>

        <Field label="Type">
          <DropMenu :values="typeOptions()" :selected="`${getParam('ft').value.Int32}`"
                    @valueClicked="setParam('ft', $event)"/>
        </Field>
        <Field label="Filter Mode">
          <DropMenu :values="filterModeOptions()" :selected="`${getParam('fm').value.Int32}`"
                    @valueClicked="setParam('fm', $event)"/>
        </Field>
        <Field label="Equalizer Mode">
          <DropMenu :values="equalModeOptions()" :selected="`${getParam('mode').value.Int32}`"
                    @valueClicked="setParam('mode', $event)"/>
        </Field>
        <Field label="Equalizer Decramping">
          <DropMenu :values="decrampOptions()" :selected="`${getParam('decramp').value.Int32}`"
                    @valueClicked="setParam('decramp', $event)"/>
        </Field>
        <Field label="Slope">
          <DropMenu :values="slopeOptions()" :selected="`${getParam('s').value.Int32}`"
                    @valueClicked="setParam('s', $event)"/>
        </Field>
      </FlowItem>

      <FlowItem width="180px">
        <div class="title">Filter</div>

        <Field label="Frequency">
          <NumberInput :min="getParam('f').min" :max="getParam('f').max" :step="0.1" suffix="Hz"
                       :value="getParam('f').value.Float32"
                       @input="setParam('f', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Width">
          <NumberInput :min="getParam('w').min" :max="getParam('w').max" :step="0.01"
                       :value="getParam('w').value.Float32"
                       @input="setParam('w', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Gain">
          <NumberInput :min="-36" :max="36" :step="0.1" suffix="dB"
                       :value="getDb('g')" @input="setDbParam('g', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Quality">
          <NumberInput :min="getParam('q').min" :max="getParam('q').max" :step="0.01"
                       :value="getParam('q').value.Float32"
                       @input="setParam('q', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Balance">
          <NumberInput :min="getParam('bal').min" :max="getParam('bal').max" :step="0.1" suffix="%"
                       :value="getParam('bal').value.Float32"
                       @input="setParam('bal', $event)" :allow-empty="false"/>
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
