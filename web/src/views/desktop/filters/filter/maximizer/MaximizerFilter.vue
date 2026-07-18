<script>
import NumberInput from "@/views/desktop/filters/layout/inputs/NumberInput.vue";
import FlowLayout from "@/views/desktop/filters/layout/FlowLayout.vue";
import FlowItem from "@/views/desktop/filters/layout/FlowItem.vue";
import Field from "@/views/desktop/filters/layout/Field.vue";
import {getFilterConfig, setFilterValue} from "@/app/filters.js";

export default {
  name: "MaximizerFilter",
  components: {FlowItem, Field, FlowLayout, NumberInput},
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
  }
}
</script>

<template>
  <div style="padding: 10px">
    <FlowLayout>
      <FlowItem width="200px">
        <div class="title">Controls</div>

        <Field label="Release">
          <NumberInput :min="getParam('rel').min" :max="getParam('rel').max" :step="0.01"
                       suffix="ms"
                       :value="getParam('rel').value.Float32"
                       @input="setParam('rel', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Threshold">
          <NumberInput :min="getParam('thresh').min" :max="getParam('thresh').max" :step="0.1"
                       suffix="dB"
                       :value="getParam('thresh').value.Float32"
                       @input="setParam('thresh', $event)" :allow-empty="false"/>
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
