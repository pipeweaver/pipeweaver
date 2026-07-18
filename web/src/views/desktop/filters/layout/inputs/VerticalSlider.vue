<template>
  <div class="vslider-outer">
    <input
      type="range"
      class="vslider"
      :min="min"
      :max="max"
      :step="step"
      :value="value"
      @input="$emit('input', Number($event.target.value))"
    />
  </div>
</template>

<script>
// Vertical fader used by the Equaliser's per-band strip, mirroring EasyEffects'
// Controls.Slider { orientation: Qt.Vertical } gain fader.
//
// Uses the writing-mode based vertical range input rather than a rotated horizontal one -
// rotation makes the input's CSS "width" become its on-screen height, which can't respond
// to percentage/flex sizing from a parent container (the whole point here is letting the
// band strip stretch to fill the available modal height). writing-mode: vertical-lr with
// direction: rtl gives a genuinely vertical input whose "height" means height, so flex: 1
// on the wrapper just works.
export default {
  name: "VerticalSlider",
  emits: ["input"],

  props: {
    value: {type: Number, required: true},
    min: {type: Number, default: -36},
    max: {type: Number, default: 36},
    step: {type: Number, default: 0.01},
  },
}
</script>

<style scoped>
.vslider-outer {
  width: 28px;
  flex: 1;
  min-height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.vslider {
  writing-mode: vertical-lr;
  direction: rtl;
  width: 20px;
  height: 100%;
  margin: 0;
  accent-color: #4a7fd6;
  cursor: pointer;
}
</style>
