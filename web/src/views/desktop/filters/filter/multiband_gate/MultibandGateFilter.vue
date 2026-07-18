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
  name: "MultibandGateFilter",
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

  data() {
    return {
      activeBand: 0,
      view: 'gate', // 'gate' | 'sidechain'
      bandIndices: [0, 1, 2, 3, 4, 5, 6, 7],
    };
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

    // per-band property symbol, e.g. bp('Mute') -> 'bm_2'
    bp(field) {
      const map = {
        Enable: 'cbe',
        SplitFrequency: 'sf',
        SidechainType: 'sce',
        SidechainCustomLowcutFilter: 'sclc',
        SidechainCustomHighcutFilter: 'schc',
        GateEnable: 'ge',
        Solo: 'bs',
        Mute: 'bm',
        StereoSplitSource: 'sscs',
        SidechainSource: 'scs',
        SidechainMode: 'scm',
        SidechainLookahead: 'sla',
        SidechainReactivity: 'scr',
        SidechainLowcutFrequency: 'sclf',
        SidechainHighcutFrequency: 'schf',
        AttackTime: 'at',
        ReleaseTime: 'rt',
        Hysteresis: 'gh',
        HysteresisThreshold: 'ht',
        HysteresisZone: 'hz',
        CurveThreshold: 'gt',
        CurveZone: 'gz',
        Reduction: 'gr',
        Makeup: 'mk',
        SidechainPreamp: 'scp',
      };
      return `${map[field]}_${this.activeBand}`;
    },

    boolOptions() {
      return [{value: 'false', text: 'Off'}, {value: 'true', text: 'On'}];
    },

    gateModeOptions() {
      return [
        {value: '0', text: 'Classic'},
        {value: '1', text: 'Modern'},
        {value: '2', text: 'Linear Phase'},
      ];
    },

    envelopeBoostOptions() {
      return [
        {value: '0', text: 'None'},
        {value: '1', text: 'Pink BT'},
        {value: '2', text: 'Pink MT'},
        {value: '3', text: 'Brown BT'},
        {value: '4', text: 'Brown MT'},
      ];
    },

    sidechainInputOptions() {
      return [
        {value: '0', text: 'Internal'},
        {value: '1', text: 'Link'},
      ];
    },

    sidechainModeOptions() {
      return [
        {value: '0', text: 'Peak'},
        {value: '1', text: 'RMS'},
        {value: '2', text: 'Low-pass'},
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
  }
}
</script>

<template>
  <div style="padding: 10px">
    <!-- Card: global controls -->
    <FlowLayout>
      <FlowItem width="200px">
        <div class="title">Multiband Gate</div>

        <Field label="Operating Mode">
          <DropMenu :values="gateModeOptions()" :selected="`${getParam('mode').value.Int32}`"
                    @valueClicked="setParam('mode', $event)"/>
        </Field>
        <Field label="Sidechain Boost">
          <DropMenu :values="envelopeBoostOptions()" :selected="`${getParam('envb').value.Int32}`"
                    @valueClicked="setParam('envb', $event)"/>
        </Field>
        <Field label="Stereo Split" row>
          <Toggle :value="getParam('ssplit').value.Bool" @input="setParam('ssplit', $event)"/>
        </Field>
        <Field label="Dry">
          <NumberInput :min="-80" :max="20" :step="0.1" suffix="dB"
                       :value="getDb('g_dry')" @input="setDbParam('g_dry', $event)"
                       :allow-empty="false"/>
        </Field>
        <Field label="Wet">
          <NumberInput :min="-80" :max="20" :step="0.1" suffix="dB"
                       :value="getDb('g_wet')" @input="setDbParam('g_wet', $event)"
                       :allow-empty="false"/>
        </Field>
      </FlowItem>

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

    <!-- Band selector -->
    <div class="band-tabs">
      <button v-for="i in bandIndices" :key="i" :class="{active: activeBand === i}"
              @click="activeBand = i">
        Band {{ i + 1 }}
      </button>
    </div>

    <div class="view-tabs">
      <button :class="{active: view === 'gate'}" @click="view = 'gate'">Gate</button>
      <button :class="{active: view === 'sidechain'}" @click="view = 'sidechain'">Sidechain</button>
    </div>

    <ActionBar :teleport="false">
      <ActionBarItem label="Mute" :active="getParam(bp('Mute')).value.Bool"
                     @click="setParam(bp('Mute'), `${!getParam(bp('Mute')).value.Bool}`)"/>
      <ActionBarItem label="Solo" :active="getParam(bp('Solo')).value.Bool"
                     @click="setParam(bp('Solo'), `${!getParam(bp('Solo')).value.Bool}`)"/>
      <ActionBarItem label="Bypass" :active="!getParam(bp('GateEnable')).value.Bool"
                     @click="setParam(bp('GateEnable'), `${!getParam(bp('GateEnable')).value.Bool}`)"/>
      <ActionBarItem v-if="activeBand > 0" label="Enabled"
                     :active="getParam(bp('Enable')).value.Bool"
                     @click="setParam(bp('Enable'), `${!getParam(bp('Enable')).value.Bool}`)"/>
    </ActionBar>

    <!-- Band: Gate view -->
    <FlowLayout v-if="view === 'gate'">
      <FlowItem width="160px">
        <div class="title">Frequency</div>
        <Field label="Start" v-if="activeBand > 0">
          <NumberInput :min="getParam(bp('SplitFrequency')).min"
                       :max="getParam(bp('SplitFrequency')).max" :step="1" suffix="Hz"
                       :value="getParam(bp('SplitFrequency')).value.Float32"
                       @input="setParam(bp('SplitFrequency'), $event)" :allow-empty="false"/>
        </Field>
        <Field label="Start" v-else>
          <div class="static-value">0 Hz (fixed)</div>
        </Field>
      </FlowItem>

      <FlowItem width="180px">
        <div class="title">Reaction</div>
        <div class="fields-grid">
          <Field label="Attack">
            <NumberInput :min="getParam(bp('AttackTime')).min" :max="getParam(bp('AttackTime')).max"
                         :step="0.1" suffix="ms"
                         :value="getParam(bp('AttackTime')).value.Float32"
                         @input="setParam(bp('AttackTime'), $event)" :allow-empty="false"/>
          </Field>
          <Field label="Release">
            <NumberInput :min="getParam(bp('ReleaseTime')).min"
                         :max="getParam(bp('ReleaseTime')).max"
                         :step="0.1" suffix="ms"
                         :value="getParam(bp('ReleaseTime')).value.Float32"
                         @input="setParam(bp('ReleaseTime'), $event)" :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>

      <FlowItem width="180px">
        <div class="title">Gain</div>
        <div class="fields-grid">
          <Field label="Reduction">
            <NumberInput :min="getParam(bp('Reduction')).min" :max="getParam(bp('Reduction')).max"
                         :step="0.1" suffix="dB"
                         :value="getParam(bp('Reduction')).value.Float32"
                         @input="setParam(bp('Reduction'), $event)" :allow-empty="false"/>
          </Field>
          <Field label="Makeup">
            <NumberInput :min="getParam(bp('Makeup')).min" :max="getParam(bp('Makeup')).max"
                         :step="0.1" suffix="dB"
                         :value="getParam(bp('Makeup')).value.Float32"
                         @input="setParam(bp('Makeup'), $event)" :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>

      <FlowItem width="180px">
        <div class="title">Curve</div>
        <div class="fields-grid">
          <Field label="Threshold">
            <NumberInput :min="getParam(bp('CurveThreshold')).min"
                         :max="getParam(bp('CurveThreshold')).max" :step="0.1" suffix="dB"
                         :value="getParam(bp('CurveThreshold')).value.Float32"
                         @input="setParam(bp('CurveThreshold'), $event)" :allow-empty="false"/>
          </Field>
          <Field label="Zone">
            <NumberInput :min="getParam(bp('CurveZone')).min" :max="getParam(bp('CurveZone')).max"
                         :step="0.1" suffix="dB"
                         :value="getParam(bp('CurveZone')).value.Float32"
                         @input="setParam(bp('CurveZone'), $event)" :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>

      <FlowItem width="180px">
        <div class="title">Hysteresis</div>
        <Field label="Enable" row>
          <Toggle :value="getParam(bp('Hysteresis')).value.Bool"
                  @input="setParam(bp('Hysteresis'), $event)"/>
        </Field>
        <div class="fields-grid">
          <Field label="Threshold" :disabled="!getParam(bp('Hysteresis')).value.Bool">
            <NumberInput :min="getParam(bp('HysteresisThreshold')).min"
                         :max="getParam(bp('HysteresisThreshold')).max" :step="0.1" suffix="dB"
                         :value="getParam(bp('HysteresisThreshold')).value.Float32"
                         @input="setParam(bp('HysteresisThreshold'), $event)" :allow-empty="false"/>
          </Field>
          <Field label="Zone" :disabled="!getParam(bp('Hysteresis')).value.Bool">
            <NumberInput :min="getParam(bp('HysteresisZone')).min"
                         :max="getParam(bp('HysteresisZone')).max" :step="0.1" suffix="dB"
                         :value="getParam(bp('HysteresisZone')).value.Float32"
                         @input="setParam(bp('HysteresisZone'), $event)" :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>
    </FlowLayout>

    <!-- Band: Sidechain view -->
    <FlowLayout v-else>
      <FlowItem width="200px">
        <div class="title">Band {{ activeBand + 1 }} Sidechain</div>

        <div class="fields-grid">
          <Field label="Input" full>
            <DropMenu :values="sidechainInputOptions()"
                      :selected="`${getParam(bp('SidechainType')).value.Int32}`"
                      @valueClicked="setParam(bp('SidechainType'), $event)"/>
          </Field>
          <Field label="Mode">
            <DropMenu :values="sidechainModeOptions()"
                      :selected="`${getParam(bp('SidechainMode')).value.Int32}`"
                      @valueClicked="setParam(bp('SidechainMode'), $event)"/>
          </Field>
          <Field label="Source" v-if="!getParam('ssplit').value.Bool">
            <DropMenu :values="sidechainSourceOptions()"
                      :selected="`${getParam(bp('SidechainSource')).value.Int32}`"
                      @valueClicked="setParam(bp('SidechainSource'), $event)"/>
          </Field>
          <Field label="Source" v-else>
            <DropMenu :values="stereoSplitSourceOptions()"
                      :selected="`${getParam(bp('StereoSplitSource')).value.Int32}`"
                      @valueClicked="setParam(bp('StereoSplitSource'), $event)"/>
          </Field>
          <Field label="Preamp">
            <NumberInput :min="-80" :max="40" :step="0.1" suffix="dB"
                         :value="getDb(bp('SidechainPreamp'))"
                         @input="setDbParam(bp('SidechainPreamp'), $event)" :allow-empty="false"/>
          </Field>
          <Field label="Reactivity">
            <NumberInput :min="getParam(bp('SidechainReactivity')).min"
                         :max="getParam(bp('SidechainReactivity')).max" :step="0.1" suffix="ms"
                         :value="getParam(bp('SidechainReactivity')).value.Float32"
                         @input="setParam(bp('SidechainReactivity'), $event)" :allow-empty="false"/>
          </Field>
          <Field label="Lookahead" full>
            <NumberInput :min="getParam(bp('SidechainLookahead')).min"
                         :max="getParam(bp('SidechainLookahead')).max" :step="0.1" suffix="ms"
                         :value="getParam(bp('SidechainLookahead')).value.Float32"
                         @input="setParam(bp('SidechainLookahead'), $event)" :allow-empty="false"/>
          </Field>
        </div>
      </FlowItem>

      <FlowItem width="180px">
        <div class="title">Sidechain Filter</div>

        <Field label="Low-cut" row>
          <Toggle :value="getParam(bp('SidechainCustomLowcutFilter')).value.Bool"
                  @input="setParam(bp('SidechainCustomLowcutFilter'), $event)"/>
        </Field>
        <Field label="Low-cut Frequency"
               :disabled="!getParam(bp('SidechainCustomLowcutFilter')).value.Bool">
          <NumberInput :min="getParam(bp('SidechainLowcutFrequency')).min"
                       :max="getParam(bp('SidechainLowcutFrequency')).max" :step="1" suffix="Hz"
                       :value="getParam(bp('SidechainLowcutFrequency')).value.Float32"
                       @input="setParam(bp('SidechainLowcutFrequency'), $event)"
                       :allow-empty="false"/>
        </Field>
        <Field label="High-cut" row>
          <Toggle :value="getParam(bp('SidechainCustomHighcutFilter')).value.Bool"
                  @input="setParam(bp('SidechainCustomHighcutFilter'), $event)"/>
        </Field>
        <Field label="High-cut Frequency"
               :disabled="!getParam(bp('SidechainCustomHighcutFilter')).value.Bool">
          <NumberInput :min="getParam(bp('SidechainHighcutFrequency')).min"
                       :max="getParam(bp('SidechainHighcutFrequency')).max" :step="1" suffix="Hz"
                       :value="getParam(bp('SidechainHighcutFrequency')).value.Float32"
                       @input="setParam(bp('SidechainHighcutFrequency'), $event)"
                       :allow-empty="false"/>
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

.fields-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  column-gap: 12px;
}


.static-value {
  opacity: 0.6;
  padding: 4px 0;
}

.band-tabs, .view-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin: 12px 0;
}

.band-tabs button, .view-tabs button {
  padding: 4px 12px;
  border-radius: 6px;
  border: 1px solid #ccc;
  background-color: #222222;
  color: inherit;
  cursor: pointer;
}

.band-tabs button.active, .view-tabs button.active {
  background-color: #444444;
  font-weight: 600;
}
</style>
