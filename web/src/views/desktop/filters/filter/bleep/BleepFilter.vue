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
  name: "BleepFilter",
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
        <Field label="Enabled" row>
          <Toggle :value="getParam('enabled').value.Bool"
                  @input="setParam('enabled', $event)"/>
        </Field>

        <Field label="Frequency">
          <NumberInput :min="getParam('frequency').min" :max="getParam('frequency').max" :step="1"
                       :value="getDb('frequency')" suffix="Hz"
                       @input="setDbParam('frequency', $event)" :allow-empty="false"/>
        </Field>

        <Field label="Amplitude">
          <NumberInput :min="getParam('amplitude').min" :max="getParam('amplitude').max"
                       :step="0.01"
                       :value="getParam('amplitude').value.Float32"
                       @input="setParam('amplitude', $event)" :allow-empty="false"/>
        </Field>
      </FlowItem>
    </FlowLayout>
  </div>
</template>

<style scoped>

</style>
