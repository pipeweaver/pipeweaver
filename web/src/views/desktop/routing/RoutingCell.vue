<template>
  <td role="checkbox" tabindex="0" @click="clicked">
    <font-awesome-icon v-if="enabled" icon="fa-solid fa-circle-check"/>
  </td>
</template>

<script>
export default {
  name: "RoutingCell",

  props: {
    source: String,
    target: String,
    enabled: Boolean,

    colour: {
      type: String,
      default: '#59b1b6'
    }
  },

  methods: {
    clicked() {
      this.$emit('clicked', this.source, this.target);
    }
  }
}
</script>

<style scoped>
td {
  padding: 0;
  border: var(--border);
  height: 100%;

  text-align: center;
  font-size: 18px;
  color: v-bind(colour);
  background: var(--mix-button-background);
  margin: 0;
  position: relative;
}

td:has(>input[type=checkbox]:focus) {
  background-color: #49514e;
}

td:hover {
  background: var(--mix-button-muted);
}
</style>
