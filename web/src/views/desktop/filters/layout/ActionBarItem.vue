<script>
export default {
  name: "ActionBarItem",
  emits: ['click'],

  props: {
    label: {type: String, required: true},
    active: {type: Boolean, default: false},
    toggle: {type: Boolean, default: true},
    icon: {type: Array, default: null},
  },
}
</script>

<template>
  <button
    type="button"
    class="action-bar-item"
    :class="{ active: toggle && active }"
    @click="$emit('click')">
    <font-awesome-icon v-if="icon" :icon="icon" class="action-bar-icon"/>
    <span>{{ label }}</span>
  </button>
</template>

<style scoped>
.action-bar-item {
  display: flex;
  align-items: center;
  gap: 6px;

  padding: 6px 14px;

  background-color: #252a29;
  color: inherit;
  border: 1px solid #666;
  border-radius: 4px;

  font: inherit;
  cursor: pointer;
}

.action-bar-item:hover {
  background-color: #3b413f;
}

.action-bar-item.active {
  background-color: #214283;
  border-color: #345ba8;
}

.action-bar-icon {
  font-size: 0.9em;
}
</style>
