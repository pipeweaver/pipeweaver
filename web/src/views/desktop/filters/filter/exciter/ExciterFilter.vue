<script>
import NumberInput from "@/views/desktop/filters/layout/inputs/NumberInput.vue";
import Toggle from "@/views/desktop/filters/layout/inputs/Toggle.vue";
import DropMenu from "@/views/desktop/filters/layout/inputs/DropMenu.vue";
import FlowLayout from "@/views/desktop/filters/layout/FlowLayout.vue";
import FlowItem from "@/views/desktop/filters/layout/FlowItem.vue";
import Field from "@/views/desktop/filters/layout/Field.vue";
import ActionBar from "@/views/desktop/filters/layout/ActionBar.vue";
import ActionBarItem from "@/views/desktop/filters/layout/ActionBarItem.vue";
import {dbToLinear, getFilterConfig, linearToDb, setFilterValue} from "@/app/filters.js";

export default {
  name: "ExciterFilter",
  components: {
    FlowItem,
    Field,
    FlowLayout,
    DropMenu,
    NumberInput,
    Toggle,
    ActionBar,
    ActionBarItem
  },
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

    roundToStep(n, s) {
      const decimals = (s.toString().split('.')[1] ?? '').length;
      return parseFloat(parseFloat(n).toFixed(decimals));
    },


    boolOptions() {
      return [{value: 'false', text: 'Off'}, {value: 'true', text: 'On'}];
    },
  }
}
</script>

<template>
  <div style="padding: 10px">
    <FlowLayout>
      <div class="content">
        <div style="width: 100%">
          <div style="width: 100%; text-align: center">Blend Harmonics</div>
          <div
            style="display: flex; justify-content: center; align-items: center;  gap: 5px; margin-bottom: 5px;">
            <div style="width: 22px;">3rd</div>
            <div style="width: 100%">
              <input type="range" :min="getParam('blend').min" :max="getParam('blend').max"
                     :step="1"
                     :value="getParam('blend').value.Float32"
                     @input="setParam('blend', $event.target.value)"
                     id="bass_enhancer_slider" list="ticks" style="width: 100%;"/>
            </div>
            <div style="width: 22px;">2nd</div>
          </div>

          <div style="width: 100%; text-align: center">{{ getParam('blend').value.Float32 }}</div>
          <datalist id="ticks">
            <option v-for="i in 21" :key="i" :value="i - 11"/>
          </datalist>
        </div>

        <FlowItem width="100%" title="Controls">
          <Field label="Amount">
            <NumberInput :min="-100" :max="36" :step="0.1" suffix="dB"
                         :value="getDb('amount')"
                         @input="setDbParam('amount', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Harmonics">
            <NumberInput :min="getParam('drive').min" :max="getParam('drive').max" :step="0.1"
                         :value="getParam('drive').value.Float32"
                         @input="setParam('drive', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Scope">
            <NumberInput :min="getParam('freq').min" :max="getParam('freq').max" :step="1"
                         suffix="Hz"
                         :value="getParam('freq').value.Float32"
                         @input="setParam('freq', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Ceil Active" row>
            <Toggle :value="getParam('ceil_active').value.Bool"
                    @input="setParam('ceil_active', $event)"/>
          </Field>
          <Field label="Ceil" :disabled="!getParam('ceil_active').value.Bool">
            <NumberInput :min="getParam('ceil').min" :max="getParam('ceil').max" :step="1"
                         suffix="Hz"
                         :value="getParam('ceil').value.Float32"
                         @input="setParam('ceil', $event)" :allow-empty="false"/>
          </Field>
        </FlowItem>
      </div>
    </FlowLayout>

    <ActionBar>
      <ActionBarItem label="Listen" :active="getParam('listen').value.Bool"
                     @click="setParam('listen', `${!getParam('listen').value.Bool}`)"/>
    </ActionBar>
  </div>
</template>

<style scoped>
.content {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  margin-bottom: 10px;
  width: 100%;
  min-width: 300px;
  max-width: 620px;
  gap: 20px;
}
</style>
