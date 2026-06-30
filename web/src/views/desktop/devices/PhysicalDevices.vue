<script>
import PhysicalDeviceList from "@/views/desktop/devices/PhysicalDeviceList.vue";

export default {
  name: "PhysicalDevices",
  components: {PhysicalDeviceList}
}
</script>

<template>
  <div class="devices">
    <div>
      <b>Output</b>
      <PhysicalDeviceList :is-source="false"/>
    </div>
    <div>
      <b>Input</b>
      <PhysicalDeviceList :is-source="true"/>
    </div>
  </div>
</template>

<style scoped>
.devices {
  padding: 10px;
  min-height: 200px;
  display: flex;
  flex-direction: row;
  gap: 25px;
}

.devices div {
  min-width: 300px;
}
</style>
