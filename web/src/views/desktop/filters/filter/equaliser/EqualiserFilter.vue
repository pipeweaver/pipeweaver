<script>
import NumberInput from "@/views/desktop/filters/layout/inputs/NumberInput.vue";
import Toggle from "@/views/desktop/filters/layout/inputs/Toggle.vue";
import DropMenu from "@/views/desktop/filters/layout/inputs/DropMenu.vue";
import VerticalSlider from "@/views/desktop/filters/layout/inputs/VerticalSlider.vue";
import FlowLayout from "@/views/desktop/filters/layout/FlowLayout.vue";
import FlowItem from "@/views/desktop/filters/layout/FlowItem.vue";
import Field from "@/views/desktop/filters/layout/Field.vue";
import ActionBar from "@/views/desktop/filters/layout/ActionBar.vue";
import ActionBarItem from "@/views/desktop/filters/layout/ActionBarItem.vue";
import ModalOverlay from "@/views/desktop/components/ModalOverlay.vue";
import {dbToLinear, getFilterConfig, linearToDb, setFilterValue} from "@/app/filters.js";
import {is_source} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";

export default {
  name: "EqualiserFilter",
  components: {
    FlowItem,
    Field,
    FlowLayout,
    DropMenu,
    NumberInput,
    Toggle,
    VerticalSlider,
    ActionBar,
    ActionBarItem,
    ModalOverlay
  },
  props: {
    filterId: {type: String, required: true},
    filterType: {type: String, required: true}
  },

  data() {
    return {
      update_locked: false,

      channel: 'left',
      menuBand: 0,
      bandIndices: Array.from({length: 32}, (_, i) => i),
      prefix: {
        left: {
          type: 'ftl',
          mode: 'fml',
          slope: 'sl',
          solo: 'xsl',
          mute: 'xml',
          freq: 'fl',
          q: 'ql',
          width: 'wl',
          gain: 'gl'
        },
        right: {
          type: 'ftr',
          mode: 'fmr',
          slope: 'sr',
          solo: 'xsr',
          mute: 'xmr',
          freq: 'fr',
          q: 'qr',
          width: 'wr',
          gain: 'gr'
        },
      },
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

    bandSymbol(field, index) {
      return `${this.prefix[this.channel][field]}_${index}`;
    },

    getRawValue(symbol) {
      const p = this.getParam(symbol);
      if ('Bool' in p.value) return p.value.Bool;
      if ('Int32' in p.value) return p.value.Int32;
      return p.value.Float32;
    },

    setBandParam(field, index, value) {
      this.setParam(this.bandSymbol(field, index), value);
      this.mirrorBandToRight(field, index);
    },

    setBandDbParam(field, index, value) {
      this.setDbParam(this.bandSymbol(field, index), value);
      this.mirrorBandToRight(field, index);
    },

    mirrorBandToRight(field, index) {
      if (this.channel !== 'left' || !this.getParam('clink').value.Bool) return;
      const rightSymbol = `${this.prefix.right[field]}_${index}`;
      const leftValue = this.getRawValue(this.bandSymbol(field, index));
      if (this.getRawValue(rightSymbol) === leftValue) return; // already in sync
      this.setParam(rightSymbol, leftValue);
    },

    syncAllBandsToRight() {
      const fields = ['type', 'mode', 'slope', 'solo', 'mute', 'freq', 'q', 'width', 'gain'];
      for (const index of this.bandIndices) {
        for (const field of fields) {
          const leftSymbol = `${this.prefix.left[field]}_${index}`;
          const rightSymbol = `${this.prefix.right[field]}_${index}`;
          const leftValue = this.getRawValue(leftSymbol);
          if (this.getRawValue(rightSymbol) === leftValue) continue;
          this.setParam(rightSymbol, leftValue);
        }
      }
    },

    toggleLinkChannels() {
      const enabling = !this.getParam('clink').value.Bool;
      this.setParam('clink', `${enabling}`);
      if (enabling) {
        this.channel = 'left';
        this.syncAllBandsToRight();
      }
    },

    boolOptions() {
      return [{value: 'false', text: 'Off'}, {value: 'true', text: 'On'}];
    },

    modeOptions() {
      return [
        {value: '0', text: 'IIR'},
        {value: '1', text: 'FIR'},
        {value: '2', text: 'FFT'},
        {value: '3', text: 'SPM'},
      ];
    },

    decrampOptions() {
      return [
        {value: '0', text: 'Off'},
        {value: '1', text: 'x2'},
        {value: '2', text: 'x3'},
        {value: '3', text: 'x4'},
        {value: '4', text: 'x6'},
        {value: '5', text: 'x8'},
      ];
    },

    bandTypeOptions() {
      return [
        {value: '0', text: 'Off'},
        {value: '1', text: 'Bell'},
        {value: '2', text: 'Hi-pass'},
        {value: '3', text: 'Hi-shelf'},
        {value: '4', text: 'Lo-pass'},
        {value: '5', text: 'Lo-shelf'},
        {value: '6', text: 'Notch'},
        {value: '7', text: 'Resonance'},
        {value: '8', text: 'Allpass'},
        {value: '9', text: 'Bandpass'},
        {value: '10', text: 'Ladder-pass'},
        {value: '11', text: 'Ladder-rej'},
      ];
    },

    bandModeOptions() {
      return [
        {value: '0', text: 'RLC (BT)'},
        {value: '1', text: 'RLC (MT)'},
        {value: '2', text: 'BWC (BT)'},
        {value: '3', text: 'BWC (MT)'},
        {value: '4', text: 'LRX (BT)'},
        {value: '5', text: 'LRX (MT)'},
        {value: '6', text: 'APO (DR)'},
      ];
    },

    bandSlopeOptions() {
      return [
        {value: '0', text: 'x1'},
        {value: '1', text: 'x2'},
        {value: '2', text: 'x3'},
        {value: '3', text: 'x4'},
      ];
    },

    widthEnabled(index) {
      const t = this.getParam(this.bandSymbol('type', index)).value.Int32;
      return t === 9 || t === 10 || t === 11;
    },

    openBandMenu(index) {
      this.menuBand = index;
      this.$refs.bandModal.openModal(undefined, undefined);
    },

    formattedFreq(index) {
      const f = this.getParam(this.bandSymbol('freq', index)).value.Float32;
      if (f < 1000) {
        return `${f.toFixed(0)} Hz`;
      }
      return `${(f * 0.001).toFixed(1)} kHz`;
    },

    flatResponse() {
      for (const i of this.bandIndices) {
        this.setDbParam(`gl_${i}`, 0);
        this.setDbParam(`gr_${i}`, 0);
      }
    },
  }
}
</script>

<template>
  <div class="equaliser-root">
    <FlowLayout>
      <!-- Card: Equalizer global controls -->
      <FlowItem width="200px">
        <div class="title">Equalizer</div>

        <Field label="Mode">
          <DropMenu :values="modeOptions()" :selected="`${getParam('mode').value.Int32}`"
                    @valueClicked="setParam('mode', $event)"/>
        </Field>
        <Field label="Decramping">
          <DropMenu :values="decrampOptions()" :selected="`${getParam('decramp').value.Int32}`"
                    @valueClicked="setParam('decramp', $event)"/>
        </Field>
        <Field label="Balance">
          <NumberInput :min="getParam('bal').min" :max="getParam('bal').max" :step="0.1" suffix="%"
                       :value="getParam('bal').value.Float32"
                       @input="setParam('bal', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Pitch Left">
          <NumberInput :min="getParam('frqs_l').min" :max="getParam('frqs_l').max" :step="0.1"
                       suffix="st"
                       :value="getParam('frqs_l').value.Float32"
                       @input="setParam('frqs_l', $event)" :allow-empty="false"/>
        </Field>
        <Field label="Pitch Right">
          <NumberInput :min="getParam('frqs_r').min" :max="getParam('frqs_r').max" :step="0.1"
                       suffix="st"
                       :value="getParam('frqs_r').value.Float32"
                       @input="setParam('frqs_r', $event)" :allow-empty="false"/>
        </Field>
      </FlowItem>
    </FlowLayout>

    <div class="channel-tabs" v-if="!getParam('clink').value.Bool">
      <button :class="{active: channel === 'left'}" @click="channel = 'left'">Left</button>
      <button :class="{active: channel === 'right'}" @click="channel = 'right'">Right</button>
    </div>

    <div class="bands-scroll">
      <div class="band-strip" v-for="index in bandIndices" :key="index"
           :class="{off: getParam(bandSymbol('type', index)).value.Int32 === 0}">
        <div class="band-index">{{ index + 1 }}</div>

        <button class="band-menu-btn" @click="openBandMenu(index)" title="Band settings">
          <font-awesome-icon :icon="['fas', 'gear']"/>
        </button>

        <div class="band-freq">{{ formattedFreq(index) }}</div>
        <div class="band-q">Q {{ getParam(bandSymbol('q', index)).value.Float32.toFixed(2) }}</div>

        <VerticalSlider :min="-36" :max="36" :step="0.01"
                        :value="getDb(bandSymbol('gain', index))"
                        @input="setBandDbParam('gain', index, $event)"/>

        <div class="band-gain-value">{{ getDb(bandSymbol('gain', index)).toFixed(2) }}</div>
      </div>
    </div>

    <ActionBar>
      <ActionBarItem label="Link Channels" :active="getParam('clink').value.Bool"
                     @click="toggleLinkChannels"/>
      <ActionBarItem label="Flat Response" :toggle="false" @click="flatResponse"/>
    </ActionBar>

    <ModalOverlay ref="bandModal" id="equaliserBandModal" width="360px">
      <template v-slot:title>Band {{ menuBand + 1 }}</template>

      <div class="band-modal-body">
        <Field label="Frequency">
          <NumberInput :min="getParam(bandSymbol('freq', menuBand)).min"
                       :max="getParam(bandSymbol('freq', menuBand)).max" :step="1" suffix="Hz"
                       :value="getParam(bandSymbol('freq', menuBand)).value.Float32"
                       @input="setBandParam('freq', menuBand, $event)" :allow-empty="false"/>
        </Field>

        <Field label="Gain">
          <NumberInput :min="-36" :max="36" :step="0.1" suffix="dB"
                       :value="getDb(bandSymbol('gain', menuBand))"
                       @input="setBandDbParam('gain', menuBand, $event)" :allow-empty="false"/>
        </Field>

        <Field label="Q">
          <NumberInput :min="getParam(bandSymbol('q', menuBand)).min"
                       :max="getParam(bandSymbol('q', menuBand)).max" :step="0.01"
                       :value="getParam(bandSymbol('q', menuBand)).value.Float32"
                       @input="setBandParam('q', menuBand, $event)" :allow-empty="false"/>
        </Field>

        <Field label="Width" :disabled="!widthEnabled(menuBand)">
          <NumberInput :min="getParam(bandSymbol('width', menuBand)).min"
                       :max="getParam(bandSymbol('width', menuBand)).max" :step="0.01" suffix="oct"
                       :value="getParam(bandSymbol('width', menuBand)).value.Float32"
                       @input="setBandParam('width', menuBand, $event)" :allow-empty="false"/>
        </Field>

        <Field label="Mute" row>
          <Toggle :value="getParam(bandSymbol('mute', menuBand)).value.Bool"
                  @input="setBandParam('mute', menuBand, $event)"/>
        </Field>

        <Field label="Solo" row>
          <Toggle :value="getParam(bandSymbol('solo', menuBand)).value.Bool"
                  @input="setBandParam('solo', menuBand, $event)"/>
        </Field>
      </div>

      <template v-slot:footer>
        <div class="band-modal-footer">
          <Field label="Type">
            <DropMenu :values="bandTypeOptions()"
                      :selected="`${getParam(bandSymbol('type', menuBand)).value.Int32}`"
                      @valueClicked="setBandParam('type', menuBand, $event)"/>
          </Field>
          <Field label="Mode">
            <DropMenu :values="bandModeOptions()"
                      :selected="`${getParam(bandSymbol('mode', menuBand)).value.Int32}`"
                      @valueClicked="setBandParam('mode', menuBand, $event)"/>
          </Field>
          <Field label="Slope">
            <DropMenu :values="bandSlopeOptions()"
                      :selected="`${getParam(bandSymbol('slope', menuBand)).value.Int32}`"
                      @valueClicked="setBandParam('slope', menuBand, $event)"/>
          </Field>
        </div>
      </template>
    </ModalOverlay>
  </div>
</template>

<style scoped>
.equaliser-root {
  height: 100%;
  box-sizing: border-box;
  padding: 10px;
  display: flex;
  flex-direction: column;
}

.equaliser-root > .flow-layout {
  flex: 0 0 auto;
}

.title {
  font-size: 1.1em;
  font-weight: 600;
  margin-bottom: 0.6em;
}

.channel-tabs {
  flex: 0 0 auto;
  display: flex;
  gap: 6px;
  margin: 12px 0;
}

.channel-tabs button {
  padding: 4px 14px;
  border-radius: 6px;
  border: 1px solid #ccc;
  background-color: #222222;
  color: inherit;
  cursor: pointer;
}

.channel-tabs button.active {
  background-color: #444444;
  font-weight: 600;
}

.bands-scroll {
  display: flex;
  flex-direction: row;
  flex: 1;
  min-height: 0;
  gap: 6px;
  overflow-x: auto;
  padding-bottom: 8px;
}

.band-strip {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;

  border: 1px solid #3b403f;
  padding: 8px 6px;
  border-radius: 10px;
  background-color: #252a29;
  width: 56px;
  flex: 0 0 56px;
}

.band-strip.off {
  opacity: 0.45;
}

.band-index {
  flex: 0 0 auto;
  font-weight: 600;
  font-size: 0.9em;
}

.band-menu-btn {
  flex: 0 0 auto;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  border: 1px solid #666;
  background-color: #252a29;
  color: inherit;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.75em;
}

.band-menu-btn:hover {
  background-color: #3b413f;
}

.band-freq, .band-q, .band-gain-value {
  flex: 0 0 auto;
  font-size: 0.75em;
  opacity: 0.8;
  white-space: nowrap;
}

.band-gain-value {
  font-weight: 600;
  opacity: 1;
}

.band-modal-body {
  display: flex;
  flex-direction: column;
  gap: 0.6em;
}

.band-modal-body .field input {
  width: 100%;
  box-sizing: border-box;
}

.band-modal-footer {
  display: flex;
  flex-direction: row;
  gap: 8px;
  width: 100%;
  padding: 10px 20px;
}

.band-modal-footer .field {
  flex: 1;
  min-width: 0;
}
</style>

