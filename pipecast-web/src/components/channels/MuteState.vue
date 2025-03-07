<script>
export default {
  name: 'MuteState',

  props: {
    selected: { type: Boolean, required: true },
    targets: { type: Array, required: false, default: [] }
  }
}
</script>

<template>
  <div style="display: block; width: 320px">
    <button :class="{ active: selected }">M</button>
    <button :class="{ active: selected }">v</button>
    <div style="display: inline-block; padding-left: 5px">Mute to: {{ targets }}</div>
  </div>
</template>

<style scoped>
div {
  display: flex;
  flex-direction: column;
  font-size: 12px;
  color: #fff;
}

div .active {
  border: 1px solid #ff0000;
}

button {
  border: 1px solid #59b1b6;
  padding: 0;
  margin: 0;
  background-color: #3b413f;
  color: #fff;
  width: 25px;
}
</style>
