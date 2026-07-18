<script>
export default {
  name: "Field",

  props: {
    label: {type: String, required: true},
    full: {type: Boolean, default: false},  // span both columns of a parent .fields-grid
    row: {type: Boolean, default: false},   // inline label+control layout (e.g. for a Toggle)
    disabled: {type: Boolean, default: false},
  },
}
</script>

<template>
  <div class="field" :class="{'field-full': full, 'toggle-row': row, disabled: disabled}">
    <div class="field-label">{{ label }}</div>
    <slot/>
  </div>
</template>

<style scoped>
.field {
  margin-bottom: 0.6em;
}

.field-label {
  margin-bottom: 0;
}

.field :deep(input) {
  width: 100%;
  box-sizing: border-box;
}

.field.disabled {
  opacity: 0.4;
  pointer-events: none;
}

.field.toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.field.field-full {
  grid-column: 1 / -1;
}
</style>
