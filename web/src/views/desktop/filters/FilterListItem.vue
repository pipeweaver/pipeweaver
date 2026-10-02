<script>
export default {
  name: "FilterListItem",
  props: {
    filter: {type: Object, required: true},
    filterName: {type: String, required: true},
    filterEnabled: {type: Boolean, required: false}
  },
  emits: ['remove', 'select', 'enable']
}
</script>

<template>
  <div class="filter-item" @click="$emit('select', filter)">
    <div class="filter-drag-handle">
      <font-awesome-icon :icon="['fas', 'grip-vertical']"/>
    </div>
    <div class="enable">
      <input v-if="filterEnabled !== undefined" type="checkbox" :checked="filterEnabled"
             @change="$emit('enable', filter, $event.target.checked)"
             @click.stop/>
    </div>
    <div class="title">{{ filterName }}</div>
    <div class="button" @click.stop="$emit('remove', filter)">
      <font-awesome-icon icon="fa-solid fa-xmark" :fixed-width="true" title="Clear"/>
    </div>
  </div>
</template>

<style scoped>
.filter-item {
  cursor: pointer;
  padding: 10px;

  display: flex;
  align-items: center;
}

.filter-item .enable {
  width: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  margin: 0;
}

.filter-item .enable input {
  display: block;
  width: 14px;
  height: 14px;
  margin: 0 8px;
}

.filter-item .button {
  width: 16px;
  height: 16px;
}

.filter-item .title {
  flex: 1;
}

.filter-item:hover {
  background-color: #353a39;
}
</style>
