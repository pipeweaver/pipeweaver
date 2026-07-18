<script>
import NumberInput from "@/views/desktop/filters/layout/inputs/NumberInput.vue";
import DropMenu from "@/views/desktop/filters/layout/inputs/DropMenu.vue";
import FlowLayout from "@/views/desktop/filters/layout/FlowLayout.vue";
import FlowItem from "@/views/desktop/filters/layout/FlowItem.vue";
import Field from "@/views/desktop/filters/layout/Field.vue";
import ActionBar from "@/views/desktop/filters/layout/ActionBar.vue";
import ActionBarItem from "@/views/desktop/filters/layout/ActionBarItem.vue";
import {getFilterConfig, setFilterValue} from "@/app/filters.js";

export default {
  name: "LoudnessFilter",
  components: {FlowItem, Field, FlowLayout, DropMenu, NumberInput, ActionBar, ActionBarItem},
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

    modeOptions() {
      return [
        {value: '0', text: 'FFT'},
        {value: '1', text: 'IIR'},
      ];
    },

    contourOptions() {
      return [
        {value: '0', text: 'Flat'},
        {value: '1', text: 'ISO226-2003'},
        {value: '2', text: 'Fletcher-Munson'},
        {value: '3', text: 'Robinson-Dadson'},
        {value: '4', text: 'ISO226-2023'},
      ];
    },

    fftSizeOptions() {
      return [
        {value: '0', text: '256'},
        {value: '1', text: '512'},
        {value: '2', text: '1024'},
        {value: '3', text: '2048'},
        {value: '4', text: '4096'},
        {value: '5', text: '8192'},
        {value: '6', text: '16384'},
      ];
    },

    iirApproximationOptions() {
      return [
        {value: '0', text: 'Fastest'},
        {value: '1', text: 'Low'},
        {value: '2', text: 'Normal'},
        {value: '3', text: 'High'},
        {value: '4', text: 'Best'},
      ];
    },
  }
}
</script>

<template>
  <div style="padding: 10px">
    <FlowLayout>
      <FlowItem width="220px">
        <div class="title">Controls</div>

        <div class="fields-grid">
          <Field label="Mode" full>
            <DropMenu :values="modeOptions()" :selected="`${getParam('mode').value.Int32}`"
                      @valueClicked="setParam('mode', $event)"/>
          </Field>
          <Field label="Contour">
            <DropMenu :values="contourOptions()" :selected="`${getParam('std').value.Int32}`"
                      @valueClicked="setParam('std', $event)"/>
          </Field>
          <Field label="FFT Size" v-if="getParam('mode').value.Int32 === 0">
            <DropMenu :values="fftSizeOptions()" :selected="`${getParam('fft').value.Int32}`"
                      @valueClicked="setParam('fft', $event)"/>
          </Field>
          <Field label="IIR Approximation" v-if="getParam('mode').value.Int32 === 1">
            <DropMenu :values="iirApproximationOptions()"
                      :selected="`${getParam('approx').value.Int32}`"
                      @valueClicked="setParam('approx', $event)"/>
          </Field>
          <Field label="Volume">
            <NumberInput :min="getParam('volume').min" :max="getParam('volume').max" :step="0.1"
                         suffix="dB"
                         :value="getParam('volume').value.Float32"
                         @input="setParam('volume', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Clipping Range" :disabled="!getParam('hclip').value.Bool">
            <NumberInput :min="getParam('hcrange').min" :max="getParam('hcrange').max" :step="0.01"
                         suffix="dB"
                         :value="getParam('hcrange').value.Float32"
                         @input="setParam('hcrange', $event)" :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>
    </FlowLayout>

    <ActionBar>
      <ActionBarItem label="Clipping" :active="getParam('hclip').value.Bool"
                     @click="setParam('hclip', `${!getParam('hclip').value.Bool}`)"/>
    </ActionBar>
  </div>
</template>

<style scoped>
.title {
  font-size: 1.1em;
  font-weight: 600;
  margin-bottom: 0.6em;
}

.fields-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  column-gap: 12px;
}

</style>
