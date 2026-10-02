<template>
  <button
    type="button"
    class="toggle"
    :class="{on: value, disabled: disabled}"
    :disabled="disabled"
    @click="$emit('input', !value)">
    <span class="knob"/>
  </button>
</template>

<script>
// Switch-style replacement for the boolean on/off <DropMenu> used throughout the filter
// components. :value is the current boolean, @input emits the new boolean on click.
export default {
  name: "ToggleItem",
  emits: ["input"],

  props: {
    value: {type: Boolean, default: false},
    disabled: {type: Boolean, default: false},
  },
}
</script>

<style scoped>
.toggle {
  position: relative;
  width: 38px;
  height: 20px;
  padding: 0;
  border: 1px solid #666;
  border-radius: 10px;
  background-color: #252a29;
  cursor: pointer;
  flex-shrink: 0;
}

.toggle:hover {
  border-color: #888;
}

.toggle.on {
  background-color: #214283;
  border-color: #345ba8;
}

.toggle.disabled {
  opacity: 0.4;
  pointer-events: none;
}

.knob {
  position: absolute;
  top: 1px;
  left: 1px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background-color: #ccc;
  transition: left 0.12s ease;
}

.toggle.on .knob {
  left: 19px;
  background-color: #fff;
}
</style>
