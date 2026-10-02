<script>
import {dbToLinear, getFilterConfig, linearToDb, setFilterValue} from "@/app/filters.js";
import ActionBarItem from "@/views/desktop/filters/layout/ActionBarItem.vue";
import ActionBar from "@/views/desktop/filters/layout/ActionBar.vue";
import Toggle from "@/views/desktop/filters/layout/inputs/Toggle.vue";
import NumberInput from "@/views/desktop/filters/layout/inputs/NumberInput.vue";
import FlowLayout from "@/views/desktop/filters/layout/FlowLayout.vue";
import Field from "@/views/desktop/filters/layout/Field.vue";
import FlowItem from "@/views/desktop/filters/layout/FlowItem.vue";

export default {
  name: "NoiseReductionFilter",
  components: {FlowItem, Field, FlowLayout, NumberInput, Toggle, ActionBar, ActionBarItem},
  props: {
    filterId: {type: String, required: true},
    filterType: {type: String, required: true}
  },

  methods: {
    getParam(symbol) {
      return getFilterConfig(this.filterId).parameters.find(p => p.symbol === symbol);
    },

    setParam(symbol, value) {
      console.log(symbol, value);
      setFilterValue(this.filterId, symbol, value);
    },

    setDbParam(symbol, value) {
      this.setParam(symbol, dbToLinear(value));
    },

    getDb(symbol) {
      return linearToDb(this.getParam(symbol).value.Float32);
    },
  }
}
</script>

<template>
  <div style="padding: 10px">
    <FlowLayout>
      <FlowItem width="100%" max-width="500px" min-width="250px" title="Controls">
        <Field label="Max Attenuation">
          <NumberInput :min="getParam('max_attenuation').min" :max="getParam('max_attenuation').max"
                       :step="0.1" suffix="dB"
                       :value="getDb('max_attenuation')"
                       @input="setDbParam('max_attenuation', $event)" :allow-empty="false"/>
        </Field>

        <Field label="VAD Threshold">
          <NumberInput :min="getParam('vad_threshold').min" :max="getParam('vad_threshold').max"
                       :step="0.01"
                       :value="getParam('vad_threshold').value.Float32"
                       @input="setParam('vad_threshold', $event)" :allow-empty="false"/>
        </Field>
        <Field label="VAD Grace Period">
          <NumberInput :min="getParam('vad_grace_period').min"
                       :max="getParam('vad_grace_period').max"
                       :step="1"
                       :value="getParam('vad_grace_period').value.Float32"
                       @input="setParam('vad_grace_period', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Retroactive VAD Grace Period">
          <NumberInput :min="getParam('retroactive_vad_grace_period').min"
                       :max="getParam('retroactive_vad_grace_period').max"
                       :step="1"
                       :value="getParam('retroactive_vad_grace_period').value.Float32"
                       @input="setParam('retroactive_vad_grace_period', $event)"
                       :allow-empty="false"/>
        </Field>
        <Field label="Mix">
          <NumberInput :min="getParam('mix').min"
                       :max="getParam('mix').max"
                       :step="0.01"
                       :value="getParam('mix').value.Float32"
                       @input="setParam('mix', $event)"
                       :allow-empty="false"/>
        </Field>
        <Field label="Gate Range">
          <NumberInput :min="getParam('gate_range').min"
                       :max="getParam('gate_range').max"
                       :step="0.1" suffix="dB"
                       :value="getParam('gate_range').value.Float32"
                       @input="setParam('gate_range', $event)"
                       :allow-empty="false"/>
        </Field>
        <Field label="Gate Attack">
          <NumberInput :min="getParam('gate_attack').min"
                       :max="getParam('gate_attack').max"
                       :step="1" suffix="ms"
                       :value="getParam('gate_attack').value.Float32"
                       @input="setParam('gate_attack', $event)"
                       :allow-empty="false"/>
        </Field>
        <Field label="Gate Release">
          <NumberInput :min="getParam('gate_release').min"
                       :max="getParam('gate_release').max"
                       :step="1" suffix="ms"
                       :value="getParam('gate_release').value.Float32"
                       @input="setParam('gate_release', $event)"
                       :allow-empty="false"/>
        </Field>
        <Field label="Side Reduction">
          <NumberInput :min="getParam('side_reduction').min"
                       :max="getParam('side_reduction').max"
                       :step="0.01"
                       :value="getParam('side_reduction').value.Float32"
                       @input="setParam('side_reduction', $event)"
                       :allow-empty="false"/>
        </Field>
      </FlowItem>
    </FlowLayout>
  </div>
</template>

<style scoped>

</style>
