<script>
export default {
  name: "FilterListItem",
  props: {
    filter: {type: Object, required: true},
    filterName: {type: String, required: true}
  },
  emits: ['remove', 'select']
}
</script>

<template>
  <div class="filter-item" @click="$emit('select', filter)">
    <div class="title">{{ filterName }}</div>
    <div class="button" @click.stop="$emit('remove', filter)">x</div>
  </div>
</template>

<style scoped>
.filter-item {
  cursor: pointer;
  padding: 10px;
  display: flex;
}

.filter-item .title {
  flex: 1;
}

.filter-item:hover {
  background-color: #353a39;
}
</style>
