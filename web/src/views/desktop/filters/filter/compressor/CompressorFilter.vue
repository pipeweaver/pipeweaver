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
  name: "CompressorFilter",
  components: {FlowItem, Field, FlowLayout, DropMenu, NumberInput, ActionBar, ActionBarItem},
  props: {
    filterId: {type: String, required: true},
    filterType: {type: String, required: true}
  },

  computed: {
    activeMode() {
      return this.getParam('cm').value.Int32;
    },
  },

  methods: {
    linearToDb,

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


    modeOptions() {
      return [
        {value: '0', text: 'Downward'},
        {value: '1', text: 'Upward'},
        {value: '2', text: 'Boosting'},
      ];
    },

    sidechainTypeOptions() {
      return [
        {value: '0', text: 'Feed-forward'},
        {value: '1', text: 'Feed-back'},
      ];
    },

    sidechainModeOptions() {
      return [
        {value: '0', text: 'Peak'},
        {value: '1', text: 'RMS'},
        {value: '2', text: 'Low-Pass'},
        {value: '3', text: 'SMA'},
      ];
    },

    sidechainSourceOptions() {
      return [
        {value: '0', text: 'Middle'},
        {value: '1', text: 'Side'},
        {value: '2', text: 'Left'},
        {value: '3', text: 'Right'},
        {value: '4', text: 'Min'},
        {value: '5', text: 'Max'},
      ];
    },

    stereoSplitSourceOptions() {
      return [
        {value: '0', text: 'Left/Right'},
        {value: '1', text: 'Right/Left'},
        {value: '2', text: 'Mid/Side'},
        {value: '3', text: 'Side/Mid'},
        {value: '4', text: 'Min'},
        {value: '5', text: 'Max'},
      ];
    },

    filterModeOptions() {
      return [
        {value: '0', text: 'Off'},
        {value: '1', text: '12 dB/oct'},
        {value: '2', text: '24 dB/oct'},
        {value: '3', text: '36 dB/oct'},
      ];
    },
  }
}
</script>

<template>
  <div style="padding: 10px">
    <FlowLayout>
      <!-- Card: Compressor -->
      <FlowItem width="220px">
        <div class="title">Compressor</div>

        <div class="fields-grid">
          <Field label="Mode" full>
            <DropMenu :values="modeOptions()" :selected="`${activeMode}`"
                      @valueClicked="setParam('cm', $event)"/>
          </Field>

          <Field label="Boost Threshold" v-if="activeMode === 1" full>
            <NumberInput :min="-120" :max="-60" :step="0.1" suffix="dB"
                         :value="getDb('bth')" @input="setDbParam('bth', $event)"
                         :allow-empty="false"/>
          </Field>

          <Field label="Boost Amount" v-if="activeMode === 2" full>
            <NumberInput :min="-72" :max="72" :step="0.1" suffix="dB"
                         :value="getDb('bsa')" @input="setDbParam('bsa', $event)"
                         :allow-empty="false"/>
          </Field>

          <Field label="Ratio">
            <NumberInput :min="getParam('cr').min" :max="getParam('cr').max" :step="0.1"
                         :value="getParam('cr').value.Float32"
                         @input="setParam('cr', $event)" :allow-empty="false"/>
          </Field>

          <Field label="Knee">
            <NumberInput :min="-24" :max="0" :step="0.1" suffix="dB"
                         :value="getDb('kn')" @input="setDbParam('kn', $event)"
                         :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>

      <!-- Card: Threshold and Time -->
      <FlowItem width="220px">
        <div class="title">Threshold and Time</div>

        <div class="fields-grid">
          <Field label="Threshold">
            <NumberInput :min="-60" :max="0" :step="0.1" suffix="dB"
                         :value="getDb('al')" @input="setDbParam('al', $event)"
                         :allow-empty="false"/>
          </Field>
          <Field label="Attack">
            <NumberInput :min="getParam('at').min" :max="getParam('at').max" :step="1" suffix="ms"
                         :value="getParam('at').value.Float32"
                         @input="setParam('at', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Release Threshold">
            <NumberInput :min="-80" :max="0" :step="0.1" suffix="dB"
                         :value="getDb('rrl')" @input="setDbParam('rrl', $event)"
                         :allow-empty="false"/>
          </Field>
          <Field label="Release">
            <NumberInput :min="getParam('rt').min" :max="getParam('rt').max" :step="1" suffix="ms"
                         :value="getParam('rt').value.Float32"
                         @input="setParam('rt', $event)" :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>

      <!-- Card: Sidechain -->
      <FlowItem width="220px">
        <div class="title">Sidechain</div>

        <div class="fields-grid">
          <Field label="Type" full>
            <DropMenu :values="sidechainTypeOptions()" :selected="`${getParam('sct').value.Int32}`"
                      @valueClicked="setParam('sct', $event)"/>
          </Field>
          <Field label="Mode">
            <DropMenu :values="sidechainModeOptions()" :selected="`${getParam('scm').value.Int32}`"
                      @valueClicked="setParam('scm', $event)"/>
          </Field>
          <Field label="Source" v-if="!getParam('ssplit').value.Bool">
            <DropMenu :values="sidechainSourceOptions()"
                      :selected="`${getParam('scs').value.Int32}`"
                      @valueClicked="setParam('scs', $event)"/>
          </Field>
          <Field label="Source" v-else>
            <DropMenu :values="stereoSplitSourceOptions()"
                      :selected="`${getParam('sscs').value.Int32}`"
                      @valueClicked="setParam('sscs', $event)"/>
          </Field>
        </div>
      </FlowItem>

      <!-- Card: Sidechain (filter/reactivity) -->
      <FlowItem width="220px">
        <div class="title">Sidechain Filter</div>

        <div class="fields-grid">
          <Field label="Preamp">
            <NumberInput :min="-80" :max="40" :step="0.1" suffix="dB"
                         :value="getDb('scp')" @input="setDbParam('scp', $event)"
                         :allow-empty="false"/>
          </Field>
          <Field label="Reactivity">
            <NumberInput :min="getParam('scr').min" :max="getParam('scr').max" :step="0.1"
                         suffix="ms"
                         :value="getParam('scr').value.Float32"
                         @input="setParam('scr', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Lookahead" full>
            <NumberInput :min="getParam('sla').min" :max="getParam('sla').max" :step="0.1"
                         suffix="ms"
                         :value="getParam('sla').value.Float32"
                         @input="setParam('sla', $event)" :allow-empty="false"/>
          </Field>
          <Field label="High-pass">
            <DropMenu :values="filterModeOptions()" :selected="`${getParam('shpm').value.Int32}`"
                      @valueClicked="setParam('shpm', $event)"/>
          </Field>
          <Field label="Low-pass">
            <DropMenu :values="filterModeOptions()" :selected="`${getParam('slpm').value.Int32}`"
                      @valueClicked="setParam('slpm', $event)"/>
          </Field>
          <Field label="High-pass Frequency" v-if="getParam('shpm').value.Int32 !== 0">
            <NumberInput :min="getParam('shpf').min" :max="getParam('shpf').max" :step="1"
                         suffix="Hz"
                         :value="getParam('shpf').value.Float32"
                         @input="setParam('shpf', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Low-pass Frequency" v-if="getParam('slpm').value.Int32 !== 0">
            <NumberInput :min="getParam('slpf').min" :max="getParam('slpf').max" :step="1"
                         suffix="Hz"
                         :value="getParam('slpf').value.Float32"
                         @input="setParam('slpf', $event)" :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>

      <!-- Card: Output -->
      <FlowItem width="180px">
        <div class="title">Output</div>

        <Field label="Dry">
          <NumberInput :min="-80" :max="20" :step="0.1" suffix="dB"
                       :value="getDb('cdr')" @input="setDbParam('cdr', $event)"
                       :allow-empty="false"/>
        </Field>
        <Field label="Wet">
          <NumberInput :min="-80" :max="20" :step="0.1" suffix="dB"
                       :value="getDb('cwt')" @input="setDbParam('cwt', $event)"
                       :allow-empty="false"/>
        </Field>
        <Field label="Makeup">
          <NumberInput :min="-60" :max="60" :step="0.1" suffix="dB"
                       :value="getDb('mk')" @input="setDbParam('mk', $event)" :allow-empty="false"/>
        </Field>
      </FlowItem>

      <!-- Card: Pre-Mix -->
      <FlowItem width="180px">
        <div class="title">Pre-Mix</div>

        <div class="fields-grid">
          <Field label="Input to Link">
            <NumberInput :min="-80" :max="40" :step="0.1" suffix="dB"
                         :value="getDb('in2lk')" @input="setDbParam('in2lk', $event)"
                         :allow-empty="false"/>
          </Field>
          <Field label="Link to Sidechain">
            <NumberInput :min="-80" :max="40" :step="0.1" suffix="dB"
                         :value="getDb('lk2sc')" @input="setDbParam('lk2sc', $event)"
                         :allow-empty="false"/>
          </Field>
          <Field label="Link to Input">
            <NumberInput :min="-80" :max="40" :step="0.1" suffix="dB"
                         :value="getDb('lk2in')" @input="setDbParam('lk2in', $event)"
                         :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>
    </FlowLayout>

    <ActionBar>
      <ActionBarItem label="Listen" :active="getParam('scl').value.Bool"
                     @click="setParam('scl', `${!getParam('scl').value.Bool}`)"/>
      <ActionBarItem label="Stereo Split" :active="getParam('ssplit').value.Bool"
                     @click="setParam('ssplit', `${!getParam('ssplit').value.Bool}`)"/>
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
