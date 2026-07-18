<template>
  <div class="flow-item" :style="styleObject">
    <slot/>
  </div>
</template>

<script>
export default {
  name: "FlowItem",

  props: {
    width: {
      type: String,
      default: "auto"
    },
    minWidth: {
      type: String,
      default: "auto"
    },
    maxWidth: {
      type: String,
      default: "none"
    }
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
  border: 1px solid #ccc;
  padding: 15px;
  border-radius: 10px;
  background-color: #222222;

  display: flex;
  flex-direction: column;
  align-self: stretch;
}

</style>
