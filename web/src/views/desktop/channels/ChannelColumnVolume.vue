<script>
import VerticalRange from '@/views/desktop/inputs/VerticalRange.vue'

export default {
  name: 'ChannelColumnVolume',

  components: {VerticalRange},
  data() {
    return {
      localFieldValue: 0
    }
  },

  props: {
    height: {type: Number, required: false, default: 440},
    currentValue: {type: Number, required: true},
    colour1: {type: String, default: '#00ffff'},
    colour2: {type: String, default: '#252927'}
  },

  methods: {
    change(e) {
      this.localFieldValue = parseInt(e.target.value)
    },
    getHeight() {
      return this.height - 10;
    }
  },

  watch: {
    /**
     * Because changes can come from either the user interacting with the slider, or a reactive change coming from
     * elsewhere (Generally a value change in the Store), localFieldValue is used as a bind between them both.
     *
     * Here we watch for external changes, and update the local value to resync the slider to its new position.
     */
    currentValue: function (newValue) {
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
    <div>
      <VerticalRange
        id="channel"
        :change="change"
        :current-value="localFieldValue"
        :deselected-colour="colour2"
        :height="getHeight()"
        :max-value="100"
        :min-value="0"
        :selected-colour="colour1"
        aria-description=""
        aria-label=""
        aria-value=""
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
}

.range-label {
  color: #6e7676;
  padding-top: 2px;
  width: 32px;
  text-align: center;
}
</style>
