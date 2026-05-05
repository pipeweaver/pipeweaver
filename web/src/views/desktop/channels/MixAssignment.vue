<script>
import {Theme} from "@/app/theme.js";

export default {
  name: "MixAssignment",
  computed: {
    Theme() {
      return Theme
    }
  },

  props: {
    isMixA: {type: Boolean, required: true}
  },

  methods: {
    target_change: function (side) {
      this.$emit('target-change', side);
    }
  }
}
</script>

<template>
  <div class="radio-container">
    <!-- Sliding background -->
    <div
      :class="isMixA ? 'left' : 'right'"
      :style="{ backgroundColor: isMixA ? Theme.cyan : Theme.orange }"
      class="slider"
    ></div>

    <!-- Radio buttons -->
    <label :class="{ active: isMixA }">
      <input :checked="isMixA" type="radio" value="A" @change="target_change('A')">
      A
    </label>

    <label :class="{ active: !isMixA }">
      <input :checked="!isMixA" type="radio" value="B" @change="target_change('B')"/>
      B
    </label>
  </div>
</template>

<style scoped>
.radio-container {
  border-top: var(--border);
  border-bottom: var(--border);
  position: relative;
  display: flex;
  width: 100%;
  background-color: rgba(60, 60, 60, 1);
  overflow: hidden;
}

.slider {
  position: absolute;
  width: 50%;
  height: 100%;
  transition: all 0.3s ease;
  z-index: 0;
}

.slider.left {
  left: 0;
}

.slider.right {
  left: 50%;
}

label {
  flex: 1;
  z-index: 1;
  text-align: center;
  cursor: pointer;
  position: relative;
  user-select: none;
  transition: color 0.3s;
  align-content: center;
  font-weight: bold;
  margin-top: 6px;
  margin-bottom: 6px;
}

label input {
  display: none;
}

label.active {
  color: #000;
}

label:not(.active) {
  color: #fff;
}
</style>
