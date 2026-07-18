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
  name: "LimiterFilter",
  components: {FlowItem, Field, FlowLayout, DropMenu, NumberInput, ActionBar, ActionBarItem},
  props: {
    filterId: {type: String, required: true},
    filterType: {type: String, required: true}
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
        {value: '0', text: 'Herm Thin'},
        {value: '1', text: 'Herm Wide'},
        {value: '2', text: 'Herm Tail'},
        {value: '3', text: 'Herm Duck'},
        {value: '4', text: 'Exp Thin'},
        {value: '5', text: 'Exp Wide'},
        {value: '6', text: 'Exp Tail'},
        {value: '7', text: 'Exp Duck'},
        {value: '8', text: 'Line Thin'},
        {value: '9', text: 'Line Wide'},
        {value: '10', text: 'Line Tail'},
        {value: '11', text: 'Line Duck'},
      ];
    },

    oversamplingOptions() {
      return [
        {value: '0', text: 'None'},
        {value: '1', text: 'Half x2/16 bit'},
        {value: '2', text: 'Half x2/24 bit'},
        {value: '3', text: 'Half x3/16 bit'},
        {value: '4', text: 'Half x3/24 bit'},
        {value: '5', text: 'Half x4/16 bit'},
        {value: '6', text: 'Half x4/24 bit'},
        {value: '7', text: 'Half x6/16 bit'},
        {value: '8', text: 'Half x6/24 bit'},
        {value: '9', text: 'Half x8/16 bit'},
        {value: '10', text: 'Half x8/24 bit'},
        {value: '11', text: 'Full x2/16 bit'},
        {value: '12', text: 'Full x2/24 bit'},
        {value: '13', text: 'Full x3/16 bit'},
        {value: '14', text: 'Full x3/24 bit'},
        {value: '15', text: 'Full x4/16 bit'},
        {value: '16', text: 'Full x4/24 bit'},
        {value: '17', text: 'Full x6/16 bit'},
        {value: '18', text: 'Full x6/24 bit'},
        {value: '19', text: 'Full x8/16 bit'},
        {value: '20', text: 'Full x8/24 bit'},
        {value: '21', text: 'True Peak/16 bit'},
        {value: '22', text: 'True Peak/24 bit'},
      ];
    },

    ditheringOptions() {
      return [
        {value: '0', text: 'None'},
        {value: '1', text: '7 bit'},
        {value: '2', text: '8 bit'},
        {value: '3', text: '11 bit'},
        {value: '4', text: '12 bit'},
        {value: '5', text: '15 bit'},
        {value: '6', text: '16 bit'},
        {value: '7', text: '23 bit'},
        {value: '8', text: '24 bit'},
      ];
    },

    sidechainInputOptions() {
      return [
        {value: '0', text: 'Internal'},
        {value: '1', text: 'Link'},
      ];
    },
  }
}
</script>

<template>
  <div style="padding: 10px">
    <FlowLayout>
      <!-- Card: Mode -->
      <FlowItem width="200px">
        <div class="title">Mode</div>

        <Field label="Mode">
          <DropMenu :values="modeOptions()" :selected="`${getParam('mode').value.Int32}`"
                    @valueClicked="setParam('mode', $event)"/>
        </Field>
        <Field label="Oversampling">
          <DropMenu :values="oversamplingOptions()" :selected="`${getParam('ovs').value.Int32}`"
                    @valueClicked="setParam('ovs', $event)"/>
        </Field>
        <Field label="Dithering">
          <DropMenu :values="ditheringOptions()" :selected="`${getParam('dith').value.Int32}`"
                    @valueClicked="setParam('dith', $event)"/>
        </Field>
      </FlowItem>

      <!-- Card: Limiter -->
      <FlowItem width="180px">
        <div class="title">Limiter</div>

        <Field label="Threshold">
          <NumberInput :min="-48" :max="0" :step="0.1" suffix="dB"
                       :value="getDb('th')" @input="setDbParam('th', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Attack">
          <NumberInput :min="getParam('at').min" :max="getParam('at').max" :step="0.01" suffix="ms"
                       :value="getParam('at').value.Float32"
                       @input="setParam('at', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Release">
          <NumberInput :min="getParam('rt').min" :max="getParam('rt').max" :step="0.01" suffix="ms"
                       :value="getParam('rt').value.Float32"
                       @input="setParam('rt', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Stereo Link">
          <NumberInput :min="getParam('slink').min" :max="getParam('slink').max" :step="1"
                       suffix="%"
                       :value="getParam('slink').value.Float32"
                       @input="setParam('slink', $event)" :allow-empty="false"/>
        </Field>
      </FlowItem>

      <!-- Card: Sidechain -->
      <FlowItem width="180px">
        <div class="title">Sidechain</div>

        <div class="fields-grid">
          <Field label="Input" full>
            <DropMenu :values="sidechainInputOptions()"
                      :selected="`${getParam('extsc').value.Int32}`"
                      @valueClicked="setParam('extsc', $event)"/>
          </Field>
          <Field label="Preamp">
            <NumberInput :min="-80" :max="40" :step="0.1" suffix="dB"
                         :value="getDb('scp')" @input="setDbParam('scp', $event)"
                         :allow-empty="false"/>
          </Field>
          <Field label="Lookahead">
            <NumberInput :min="getParam('lk').min" :max="getParam('lk').max" :step="0.1" suffix="ms"
                         :value="getParam('lk').value.Float32"
                         @input="setParam('lk', $event)" :allow-empty="false"/>
          </Field>
        </div>
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

      <!-- Card: Automatic Level Regulation -->
      <FlowItem width="180px">
        <div class="title">Automatic Level Regulation</div>

        <Field label="Attack" :disabled="!getParam('alr').value.Bool">
          <NumberInput :min="getParam('alr_at').min" :max="getParam('alr_at').max" :step="0.01"
                       suffix="ms"
                       :value="getParam('alr_at').value.Float32"
                       @input="setParam('alr_at', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Release" :disabled="!getParam('alr').value.Bool">
          <NumberInput :min="getParam('alr_rt').min" :max="getParam('alr_rt').max" :step="1"
                       suffix="ms"
                       :value="getParam('alr_rt').value.Float32"
                       @input="setParam('alr_rt', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Knee" :disabled="!getParam('alr').value.Bool">
          <NumberInput :min="-12" :max="12" :step="0.1" suffix="dB"
                       :value="getDb('knee')" @input="setDbParam('knee', $event)"
                       :allow-empty="false"/>
        </Field>
        <Field label="Smooth" :disabled="!getParam('alr').value.Bool">
          <NumberInput :min="getParam('smooth').min" :max="getParam('smooth').max" :step="0.1"
                       :value="getParam('smooth').value.Float32"
                       @input="setParam('smooth', $event)" :allow-empty="false"/>
        </Field>
      </FlowItem>
    </FlowLayout>

    <ActionBar>
      <ActionBarItem label="Gain Boost" :active="getParam('boost').value.Bool"
                     @click="setParam('boost', `${!getParam('boost').value.Bool}`)"/>
      <ActionBarItem label="Automatic Level Regulation" :active="getParam('alr').value.Bool"
                     @click="setParam('alr', `${!getParam('alr').value.Bool}`)"/>
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
