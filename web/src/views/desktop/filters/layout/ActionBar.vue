<script>
export default {
  name: "ActionBar",
  inject: ['actionBarTarget'],

  props: {
    teleport: {type: Boolean, default: true},
  },
};
</script>

<template>
  <Teleport :to="actionBarTarget" v-if="teleport">
    <slot/>
  </Teleport>
  <div v-else class="action-bar-inline">
    <slot/>
  </div>
</template>

<style scoped>
.action-bar-inline {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;

  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid #3b403f;
}
</style>
