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
  name: "ReverberationFilter",
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

    roomSizeOptions() {
      return [
        {value: '0', text: 'Small'},
        {value: '1', text: 'Medium'},
        {value: '2', text: 'Large'},
        {value: '3', text: 'Tunnel-like'},
        {value: '4', text: 'Large/smooth'},
        {value: '5', text: 'Experimental'},
      ];
    },

    applyPreset(preset) {
      if (preset.decayTime !== undefined) this.setParam('decay_time', preset.decayTime);
      if (preset.hfDamp !== undefined) this.setParam('hf_damp', preset.hfDamp);
      if (preset.roomSize !== undefined) this.setParam('room_size', preset.roomSize);
      if (preset.diffusion !== undefined) this.setParam('diffusion', preset.diffusion);
      if (preset.amountLinear !== undefined) this.setDbParam('amount', linearToDb(preset.amountLinear));
      if (preset.dryLinear !== undefined) this.setDbParam('dry', linearToDb(preset.dryLinear));
      if (preset.predelay !== undefined) this.setParam('predelay', preset.predelay);
      if (preset.bassCut !== undefined) this.setParam('bass_cut', preset.bassCut);
      if (preset.trebleCut !== undefined) this.setParam('treble_cut', preset.trebleCut);
    },

    presetAmbience() {
      this.applyPreset({
        decayTime: 1.10354, hfDamp: 2182.58, roomSize: 4, diffusion: 0.69,
        amountLinear: 0.291183, dryLinear: 1, predelay: 6.5, bassCut: 514.079, trebleCut: 4064.15,
      });
    },

    presetEmptyWalls() {
      this.applyPreset({
        decayTime: 0.505687, hfDamp: 3971.64, roomSize: 4, diffusion: 0.17,
        amountLinear: 0.198884, dryLinear: 1, predelay: 13, bassCut: 240.453, trebleCut: 3303.47,
      });
    },

    presetRoom() {
      this.applyPreset({
        decayTime: 0.445945, hfDamp: 5508.46, roomSize: 4, diffusion: 0.54,
        amountLinear: 0.469761, dryLinear: 1, predelay: 25, bassCut: 257.65, trebleCut: 20000,
      });
    },

    presetLargeEmptyHall() {
      this.applyPreset({decayTime: 2.00689, hfDamp: 20000, amountLinear: 0.366022});
    },

    presetDisco() {
      this.applyPreset({decayTime: 1, hfDamp: 3396.49, amountLinear: 0.269807});
    },

    presetLargeOccupiedHall() {
      // Identical to "Disco" upstream - see class-level note above.
      this.applyPreset({decayTime: 1, hfDamp: 3396.49, amountLinear: 0.269807});
    },
  }
}
</script>

<template>
  <div style="padding: 10px">
    <FlowLayout>
      <FlowItem width="200px">
        <div class="title">Controls</div>

        <Field label="Room Size">
          <DropMenu :values="roomSizeOptions()" :selected="`${getParam('room_size').value.Int32}`"
                    @valueClicked="setParam('room_size', $event)"/>
        </Field>
        <Field label="Decay Time">
          <NumberInput :min="getParam('decay_time').min" :max="getParam('decay_time').max"
                       :step="0.01" suffix="s"
                       :value="getParam('decay_time').value.Float32"
                       @input="setParam('decay_time', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Pre Delay">
          <NumberInput :min="getParam('predelay').min" :max="getParam('predelay').max" :step="1"
                       suffix="ms"
                       :value="getParam('predelay').value.Float32"
                       @input="setParam('predelay', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Diffusion">
          <NumberInput :min="getParam('diffusion').min" :max="getParam('diffusion').max"
                       :step="0.01"
                       :value="getParam('diffusion').value.Float32"
                       @input="setParam('diffusion', $event)" :allow-empty="false"/>
        </Field>
      </FlowItem>

      <FlowItem width="180px">
        <div class="title">Filter</div>

        <div class="fields-grid">
          <Field label="High Frequency Damping" full>
            <NumberInput :min="getParam('hf_damp').min" :max="getParam('hf_damp').max" :step="1"
                         suffix="Hz"
                         :value="getParam('hf_damp').value.Float32"
                         @input="setParam('hf_damp', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Bass Cut">
            <NumberInput :min="getParam('bass_cut').min" :max="getParam('bass_cut').max" :step="1"
                         suffix="Hz"
                         :value="getParam('bass_cut').value.Float32"
                         @input="setParam('bass_cut', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Treble Cut">
            <NumberInput :min="getParam('treble_cut').min" :max="getParam('treble_cut').max"
                         :step="1" suffix="Hz"
                         :value="getParam('treble_cut').value.Float32"
                         @input="setParam('treble_cut', $event)" :allow-empty="false"/>
          </Field>
          <Field label="Dry">
            <NumberInput :min="-100" :max="6" :step="0.1" suffix="dB"
                         :value="getDb('dry')" @input="setDbParam('dry', $event)"
                         :allow-empty="false"/>
          </Field>
          <Field label="Wet">
            <NumberInput :min="-100" :max="6" :step="0.1" suffix="dB"
                         :value="getDb('amount')" @input="setDbParam('amount', $event)"
                         :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>
    </FlowLayout>

    <ActionBar>
      <ActionBarItem label="Ambience" :toggle="false" @click="presetAmbience"/>
      <ActionBarItem label="Empty Walls" :toggle="false" @click="presetEmptyWalls"/>
      <ActionBarItem label="Room" :toggle="false" @click="presetRoom"/>
      <ActionBarItem label="Large Empty Hall" :toggle="false" @click="presetLargeEmptyHall"/>
      <ActionBarItem label="Disco" :toggle="false" @click="presetDisco"/>
      <ActionBarItem label="Large Occupied Hall" :toggle="false" @click="presetLargeOccupiedHall"/>
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
