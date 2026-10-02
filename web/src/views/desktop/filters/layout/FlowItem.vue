<template>
  <div class="flow-item" :style="styleObject">
    <div class="title" v-if="title">{{ title }}</div>
    <div class="content">
      <slot/>
    </div>
  </div>
</template>

<script>
export default {
  name: "FlowItem",

  props: {
    title: {type: String, optional: true},
    width: {type: String, default: "auto"},
    minWidth: {type: String, default: "auto"},
    maxWidth: {type: String, default: "none"}
  },

  computed: {
    styleObject() {
      return {
        width: this.width,
        minWidth: this.minWidth,
        maxWidth: this.maxWidth,
      };
    }
  }
};
</script>

<style scoped>
.flow-item {
  box-sizing: border-box;
  border: 1px solid var(--border-subtle-colour);
  border-radius: 10px;
  background-color: var(--bg);
  overflow: hidden;

  display: flex;
  flex-direction: column;
  align-self: stretch;
}

.flow-item .title {
  padding: 10px;
  font-weight: bold;
  background-color: var(--panel);
  border-bottom: 1px solid var(--border-subtle-colour);
}

.flow-item .content {
  padding: 15px;
}

</style>
