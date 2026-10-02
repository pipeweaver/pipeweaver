<script>
import {dbToLinear, getFilterConfig, linearToDb, setFilterValue} from "@/app/filters.js";

export default {
  name: "BassEnhancerFilter",
  components: {},
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

    boolOptions() {
      return [{value: 'false', text: 'Off'}, {value: 'true', text: 'On'}];
    },

    roundToStep(n, s) {
      const decimals = (s.toString().split('.')[1] ?? '').length;
      return parseFloat(parseFloat(n).toFixed(decimals));
    },
  }
}
</script>

<template>
  <div class="content">
    <div style="width: 100%">
      <div style="width: 100%; text-align: center">Gain Adjustment</div>
      <div
        style="display: flex; justify-content: center; align-items: center;  gap: 5px; margin-bottom: 5px;">
        <input type="range" :min="getParam('amount').min" :max="getParam('amount').max"
               :step="0.1"
               :value="getParam('amount').value.Float32"
               @input="setParam('amount', roundToStep($event.target.value, 0.1))"
               style="width: 100%;"/>
      </div>
      <div style="width: 100%; text-align: center">{{
          roundToStep(getParam('amount').value.Float32, 0.1)
        }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.content {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;

  width: 100%;
  max-width: 800px;

  margin-left: auto;
  margin-right: auto;

  margin-bottom: 10px;
  padding: 10px;
  gap: 20px;
  box-sizing: border-box;
}
</style>
