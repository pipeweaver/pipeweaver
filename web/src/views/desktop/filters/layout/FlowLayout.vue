<template>
  <div class="flow-layout">
    <slot/>
  </div>
</template>

<script>
export default {
  name: "FlowLayout"
};
</script>

<style scoped>
.flow-layout {
  display: flex;
  flex-wrap: wrap;

  justify-content: center;

  gap: 12px;

  align-items: flex-start;
  align-content: flex-start;
}
</style>
