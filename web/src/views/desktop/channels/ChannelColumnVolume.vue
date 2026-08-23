<script>
import VerticalRange from '@/views/desktop/inputs/VerticalRange.vue'

export default {
  name: 'ChannelColumnVolume',

  components: {VerticalRange},
  data() {
    return {
      localFieldValue: 0,
      interacting: false,
    }
  },

  props: {
    id: {type: String, required: true},
    height: {type: Number, required: false, default: null},
    currentValue: {type: Number, required: true},
    colour1: {type: String, default: '#00ffff'},
    colour2: {type: String, default: '#252927'}
  },

  methods: {
    input(e) {
      this.interacting = true;
      this.localFieldValue = parseInt(e.target.value)
    },
    change(e) {
      this.localFieldValue = parseInt(e.target.value)
      this.interacting = false;
    },
    getHeight() {
      return this.height !== null ? this.height - 10 : null;
    }
  },

  watch: {
    /**
     * Because changes can come from either the user interacting with the slider, or a reactive change coming from
     * elsewhere (Generally a value change in the Store), localFieldValue is used as a bind between them both.
     *
     * Here we watch for external changes, and update the local value to resync the slider to its new position.
     * While the user is actively dragging (interacting), external updates are ignored - otherwise a reply for an
     * earlier position landing mid-drag would jerk the displayed % backwards.
     */
    currentValue: function (newValue) {
      if (this.interacting) return;
      this.localFieldValue = newValue
    }
  },

  mounted() {
    this.localFieldValue = this.currentValue
  }
}
</script>

<template>
  <div class="range">
    <div class="slider-wrap">
      <VerticalRange
        :id="id"
        :current-value="localFieldValue"
        :deselected-colour="colour2"
        :height="getHeight()"
        :max-value="100"
        :min-value="0"
        :selected-colour="colour1"
        aria-description=""
        aria-label=""
        aria-value=""
        @input="input"
        @change="change"
      />
    </div>
    <div class="range-label">{{ localFieldValue }}%</div>
  </div>


</template>

<style scoped>
.range {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex: 1;
  min-height: 0;
}

.slider-wrap {
  flex: 1;
  min-height: 0;
  width: 100%;
  display: flex;
  justify-content: center;
}

.range-label {
  color: #6e7676;
  padding-top: 2px;
  width: 32px;
  text-align: center;
}
</style>
