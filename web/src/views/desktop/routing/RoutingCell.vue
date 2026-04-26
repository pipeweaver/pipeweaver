<template>
  <td>
    <div role="checkbox" tabindex="0" @click="clicked">
      <font-awesome-icon v-if="enabled" icon="fa-solid fa-circle-check"/>
    </div>
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
}

div {
  text-align: center;
  font-size: 18px;
  color: v-bind(colour);
  background-color: #3b413f;
  margin: 0;
  position: relative;
  height: 30px;
}

/* Force SVG to center of the Cell */
div svg {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  left: 0;
  right: 0;
  margin: auto;
  text-align: center;
}

div:has(>input[type=checkbox]:focus) {
  background-color: #49514e;
}

div:hover {
  background-color: #49514e;
}
</style>
