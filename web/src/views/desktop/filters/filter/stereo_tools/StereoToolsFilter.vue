<script>
import NumberInput from "@/views/desktop/filters/layout/inputs/NumberInput.vue";
import Toggle from "@/views/desktop/filters/layout/inputs/Toggle.vue";
import DropMenu from "@/views/desktop/filters/layout/inputs/DropMenu.vue";
import FlowLayout from "@/views/desktop/filters/layout/FlowLayout.vue";
import FlowItem from "@/views/desktop/filters/layout/FlowItem.vue";
import Field from "@/views/desktop/filters/layout/Field.vue";
import {dbToLinear, getFilterConfig, linearToDb, setFilterValue} from "@/app/filters.js";

export default {
  name: "StereoToolsFilter",
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

    boolOptions() {
      return [{value: 'false', text: 'Off'}, {value: 'true', text: 'On'}];
    },

    modeOptions() {
      return [
        {value: '0', text: 'LR > LR (Stereo Default)'},
        {value: '1', text: 'LR > MS (Stereo to Mid-Side)'},
        {value: '2', text: 'MS > LR (Mid-Side to Stereo)'},
        {value: '3', text: 'LR > LL (Mono Left Channel)'},
        {value: '4', text: 'LR > RR (Mono Right Channel)'},
        {value: '5', text: 'LR > L+R (Mono Sum L+R)'},
        {value: '6', text: 'LR > RL (Stereo Flip Channels)'},
      ];
    },
  }
}
</script>

<template>
  <div style="padding: 10px">
    <FlowLayout>
      <FlowItem width="180px">
        <div class="title">Input</div>

        <Field label="Softclip" row>
          <Toggle :value="getParam('softclip').value.Bool" @input="setParam('softclip', $event)"/>
        </Field>
        <Field label="Balance">
          <NumberInput :min="getParam('balance_in').min" :max="getParam('balance_in').max"
                       :step="0.01"
                       :value="getParam('balance_in').value.Float32"
                       @input="setParam('balance_in', $event)" :allow-empty="false"/>
        </Field>
        <Field label="S/C Level" :disabled="!getParam('softclip').value.Bool">
          <NumberInput :min="getParam('sc_level').min" :max="getParam('sc_level').max" :step="0.001"
                       :value="getParam('sc_level').value.Float32"
                       @input="setParam('sc_level', $event)" :allow-empty="false"/>
        </Field>
      </FlowItem>

      <FlowItem width="200px">
        <div class="title">Stereo Matrix</div>

        <div class="fields-grid">
          <Field label="Mode" full>
            <DropMenu :values="modeOptions()" :selected="`${getParam('mode').value.Int32}`"
                      @valueClicked="setParam('mode', $event)"/>
          </Field>
          <Field label="Side Level">
            <NumberInput :min="-36" :max="36" :step="0.1" suffix="dB"
                         :value="getDb('slev')" @input="setDbParam('slev', $event)"
                         :allow-empty="false"/>
          </Field>
          <Field label="Side Balance">
            <NumberInput :min="getParam('sbal').min" :max="getParam('sbal').max" :step="0.01"
                         :value="getParam('sbal').value.Float32"
                         @input="setParam('sbal', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Middle Level">
            <NumberInput :min="-36" :max="36" :step="0.1" suffix="dB"
                         :value="getDb('mlev')" @input="setDbParam('mlev', $event)"
                         :allow-empty="false"/>
          </Field>
          <Field label="Middle Panorama">
            <NumberInput :min="getParam('mpan').min" :max="getParam('mpan').max" :step="0.01"
                         :value="getParam('mpan').value.Float32"
                         @input="setParam('mpan', $event)" :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>

      <FlowItem width="160px">
        <div class="title">Left / Right</div>

        <Field label="Left Mute" row>
          <Toggle :value="getParam('mutel').value.Bool" @input="setParam('mutel', $event)"/>
        </Field>
        <Field label="Left Invert Phase" row>
          <Toggle :value="getParam('phasel').value.Bool" @input="setParam('phasel', $event)"/>
        </Field>
        <Field label="Right Mute" row>
          <Toggle :value="getParam('muter').value.Bool" @input="setParam('muter', $event)"/>
        </Field>
        <Field label="Right Invert Phase" row>
          <Toggle :value="getParam('phaser').value.Bool" @input="setParam('phaser', $event)"/>
        </Field>
      </FlowItem>

      <FlowItem width="180px">
        <div class="title">Output</div>

        <div class="fields-grid">
          <Field label="Balance">
            <NumberInput :min="getParam('balance_out').min" :max="getParam('balance_out').max"
                         :step="0.01"
                         :value="getParam('balance_out').value.Float32"
                         @input="setParam('balance_out', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Delay">
            <NumberInput :min="getParam('delay').min" :max="getParam('delay').max" :step="0.01"
                         suffix="ms"
                         :value="getParam('delay').value.Float32"
                         @input="setParam('delay', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Stereo Base">
            <NumberInput :min="getParam('stereo_base').min" :max="getParam('stereo_base').max"
                         :step="0.01"
                         :value="getParam('stereo_base').value.Float32"
                         @input="setParam('stereo_base', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Stereo Phase">
            <NumberInput :min="getParam('stereo_phase').min" :max="getParam('stereo_phase').max"
                         :step="1" suffix="°"
                         :value="getParam('stereo_phase').value.Float32"
                         @input="setParam('stereo_phase', $event)" :allow-empty="false"/>
          </Field>
        </div>
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

.fields-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  column-gap: 12px;
}

</style>
