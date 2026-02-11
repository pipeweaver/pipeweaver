<script>
export default {
  name: "DelayChannel",
  props: {
    channel: {type: String, required: true}
  }
}
</script>

<template>
  <div>
    <div v-if="channel === 'l'">Left</div>
    <div v-else>Right</div>
  </div>
  <div>
    Mode:
    <select>
      <option>Samples</option>
      <option>Distance</option>
      <option>Time</option>
    </select>
  </div>
</template>

<style scoped>

</style>
