<script>
import NumberInput from "@/views/desktop/filters/layout/inputs/NumberInput.vue";
import DropMenu from "@/views/desktop/filters/layout/inputs/DropMenu.vue";
import FlowLayout from "@/views/desktop/filters/layout/FlowLayout.vue";
import FlowItem from "@/views/desktop/filters/layout/FlowItem.vue";
import Field from "@/views/desktop/filters/layout/Field.vue";
import ActionBar from "@/views/desktop/filters/layout/ActionBar.vue";
import ActionBarItem from "@/views/desktop/filters/layout/ActionBarItem.vue";
import {dbToLinear, getFilterConfig, linearToDb, setFilterValue} from "@/app/filters.js";

export default {
  name: "DeesserFilter",
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

    setDbParam(symbol, value) {
      this.setParam(symbol, dbToLinear(value));
    },

    getDb(symbol) {
      return linearToDb(this.getParam(symbol).value.Float32);
    },


    detectionOptions() {
      return [
        {value: '0', text: 'RMS'},
        {value: '1', text: 'Peak'},
      ];
    },

    modeOptions() {
      return [
        {value: '0', text: 'Wideband'},
        {value: '1', text: 'Split'},
      ];
    },
  }
}
</script>

<template>
  <div style="padding: 10px">
    <FlowLayout>
      <FlowItem width="220px">
        <div class="title">Deesser</div>

        <div class="fields-grid">
          <Field label="Detection">
            <DropMenu :values="detectionOptions()"
                      :selected="`${getParam('detection').value.Int32}`"
                      @valueClicked="setParam('detection', $event)"/>
          </Field>
          <Field label="Mode">
            <DropMenu :values="modeOptions()" :selected="`${getParam('mode').value.Int32}`"
                      @valueClicked="setParam('mode', $event)"/>
          </Field>
          <Field label="Threshold">
            <NumberInput :min="-60" :max="0" :step="0.1" suffix="dB"
                         :value="getDb('threshold')" @input="setDbParam('threshold', $event)"
                         :allow-empty="false"/>
          </Field>
          <Field label="Ratio">
            <NumberInput :min="getParam('ratio').min" :max="getParam('ratio').max" :step="0.01"
                         :value="getParam('ratio').value.Float32"
                         @input="setParam('ratio', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Makeup">
            <NumberInput :min="0" :max="36" :step="0.1" suffix="dB"
                         :value="getDb('makeup')" @input="setDbParam('makeup', $event)"
                         :allow-empty="false"/>
          </Field>
          <Field label="Laxity">
            <NumberInput :min="getParam('laxity').min" :max="getParam('laxity').max" :step="1"
                         :value="getParam('laxity').value.Int32"
                         @input="setParam('laxity', $event)" :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>

      <FlowItem width="180px">
        <div class="title">Filter</div>

        <div class="fields-grid">
          <Field label="F1 Split">
            <NumberInput :min="getParam('f1_freq').min" :max="getParam('f1_freq').max" :step="1"
                         suffix="Hz"
                         :value="getParam('f1_freq').value.Float32"
                         @input="setParam('f1_freq', $event)" :allow-empty="false"/>
          </Field>
          <Field label="F2 Peak">
            <NumberInput :min="getParam('f2_freq').min" :max="getParam('f2_freq').max" :step="1"
                         suffix="Hz"
                         :value="getParam('f2_freq').value.Float32"
                         @input="setParam('f2_freq', $event)" :allow-empty="false"/>
          </Field>
          <Field label="F1 Gain">
            <NumberInput :min="-24" :max="24" :step="0.1" suffix="dB"
                         :value="getDb('f1_level')" @input="setDbParam('f1_level', $event)"
                         :allow-empty="false"/>
          </Field>
          <Field label="F2 Level">
            <NumberInput :min="-24" :max="24" :step="0.1" suffix="dB"
                         :value="getDb('f2_level')" @input="setDbParam('f2_level', $event)"
                         :allow-empty="false"/>
          </Field>
          <Field label="F2 Peak Q" full>
            <NumberInput :min="getParam('f2_q').min" :max="getParam('f2_q').max" :step="0.01"
                         :value="getParam('f2_q').value.Float32"
                         @input="setParam('f2_q', $event)" :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>
    </FlowLayout>

    <ActionBar>
      <ActionBarItem label="Listen" :active="getParam('sc_listen').value.Bool"
                     @click="setParam('sc_listen', `${!getParam('sc_listen').value.Bool}`)"/>
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
