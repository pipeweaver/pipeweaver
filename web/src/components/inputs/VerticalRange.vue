<script>
/**
 * This is simply a vertical range slider component. While there's finally a spec for doing this (as of 18/04/2024),
 * it's not implemented in all browsers and other general workarounds for this have severe limitations around things
 * like styling.
 *
 * So Fuck It.
 *
 * So we're simply going to rotate -90deg, and use Javascript to correctly position the input into a correctly fitting
 * div which can be used by the parent, saving us from having to do bullshit workarounds and 'fixes' to get this
 * working correctly.
 */

export default {
  name: 'VerticalRange',

  props: {
    height: {type: Number, required: true, default: 120},

    // Minimum Value for the Slider
    minValue: {type: Number, required: true, default: 0},

    // Maximum Value for the Slider
    maxValue: {type: Number, required: true, default: 100},

    // The current value of the Slider
    currentValue: {type: Number, required: true, default: 20},

    // The stepping of the input.
    step: {type: Number, required: false, default: 1},

    // Whether the control is disabled
    disabled: {type: Boolean, required: false, default: false},

    // A Unique Identifier for reporting value changes
    id: {type: String, required: true},

    // Colours for the thumb and 'active' section, and the unselected colour
    selectedColour: {type: String, required: false, default: '#82CFD0'},
    deselectedColour: {type: String, required: false, default: '#000000'},

    // The value to report to Screen Readers
    ariaValue: {type: String, required: true},
    ariaLabel: {type: String, required: true},
    ariaDescription: {type: String, required: true}
  },

  data() {
    return {
      localFieldValue: 0
    }
  },

  methods: {
    calc_position: function () {
      // Half outer width minus half range width
      return this.height - (16 / 2 - 6 / 2) - 2
    },

    hexToRgb: function (hex) {
      // Expand shorthand form (e.g. "03F") to full form (e.g. "0033FF")
      let shorthandRegex = /^#?([a-f\d])([a-f\d])([a-f\d])$/i
      hex = hex.replace(shorthandRegex, function (m, r, g, b) {
        return r + r + g + g + b + b
      })

      let result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex)
      return result
        ? {
          r: parseInt(result[1], 16),
          g: parseInt(result[2], 16),
          b: parseInt(result[3], 16)
        }
        : null
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

  computed: {
    calc_height() {
      return this.height + 'px'
    },
    calc_transform() {
      return `rotate(-90deg) translateY(-${this.calc_position()}px)`
    },
    glow_value() {
      let rgb = this.hexToRgb(this.selectedColour)
      return `0 0 0 10px rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.2)`
    },

    currentWidth() {
      // This code essentially adjusts the background position to keep it below the 'thumb'..
      let distance = this.maxValue - this.minValue
      let position = 0

      for (let i = this.minValue; i <= this.maxValue; i += this.step, position += this.step) {
        if (i === parseFloat(this.localFieldValue)) {
          break
        }
      }

      let width = (position / distance) * 100
      if (isNaN(width)) {
        return '0%'
      }
      return width + '%'
    }
  },

  mounted() {
    this.localFieldValue = this.currentValue
  }
}
</script>

<template>
  <div class="outer">
    <input
      v-model="localFieldValue"
      :aria-description="ariaDescription"
      :aria-label="ariaLabel"
      :aria-valuetext="ariaValue"
      :disabled="disabled"
      :max="maxValue"
      :min="minValue"
      :step="step"

      type="range"
    />
  </div>
</template>

<style scoped>
.outer {
  width: 20px;
  height: v-bind(calc_height);
}

input[type='range'] {

  background: linear-gradient(
    to right,
    v-bind(selectedColour) 0%,
    v-bind(selectedColour) v-bind(currentWidth),
    v-bind(deselectedColour) v-bind(currentWidth),
    v-bind(deselectedColour) 100%
  );


  /*
    If we ever were to do metering in the UI, we could use something like this to attach it
    directly to the range bar:

    TODO: This doesn't work, metering is too heavy to redraw a linear gradient

    background: linear-gradient(
      to right,
      v-bind(meterColour) 0%,
      v-bind(meterColour) 20%,
      v-bind(selectedColour) 20%,
      v-bind(selectedColour) v-bind(currentWidth),
      v-bind(deselectedColour) v-bind(currentWidth),
      v-bind(deselectedColour) 100%
    );
  */

  display: block;
  transform-origin: top right;
  transform: v-bind(calc_transform);
  -webkit-appearance: none;
  appearance: none;

  width: v-bind(calc_height);
  cursor: pointer;
  outline: none;
  border-radius: 15px;

  margin: 0;

  height: 6px;
}

/* Thumb: webkit */
input[type='range']::-webkit-slider-thumb {
  /* removing default appearance */
  -webkit-appearance: none;
  appearance: none;

  height: 16px;
  width: 16px;
  background: v-bind(selectedColour);
  border-radius: 50%;
  border: none;

  transition: 0.2s ease-in-out;
}

/* Thumb: Firefox */
input[type='range']::-moz-range-thumb {
  height: 16px;
  width: 16px;
  background: v-bind(selectedColour);
  border-radius: 50%;
  border: none;

  transition: 0.2s ease-in-out;
}

/* Hover, active & focus Thumb: Webkit */
input[type='range']::-webkit-slider-thumb:hover, input[type='range']::-webkit-slider-thumb:active {
  box-shadow: v-bind(glow_value);
}

/* Hover, active & focus Thumb: Firefox */
input[type='range']::-moz-range-thumb:hover, input[type='range']::-moz-range-thumb:active {
  box-shadow: v-bind(glow_value);
}
</style>
