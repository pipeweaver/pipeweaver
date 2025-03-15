<script setup>

import {DeviceType, get_devices} from "@/pipecast/util.js";
import ChannelColumn from "@/components/channels/ChannelColumn.vue";
</script>

<template>
  <div class="wrapper">
    <div class="mixer">
      <div v-for="id in get_devices(DeviceType.PhysicalSource).keys()">
        <ChannelColumn :index=id :type='DeviceType.PhysicalSource'/>
      </div>
    </div>
    <div class="mixer">
      <div v-for="id in get_devices(DeviceType.VirtualSource).keys()">
        <ChannelColumn :index=id :type='DeviceType.VirtualSource'/>
      </div>
    </div>

    <div class="mixer">
      <div v-for="id in get_devices(DeviceType.PhysicalTarget).keys()">
        <ChannelColumn :index=id :type='DeviceType.PhysicalTarget'/>
      </div>
    </div>
    <div class="mixer">
      <div v-for="id in get_devices(DeviceType.VirtualTarget).keys()">
        <ChannelColumn :index=id :type='DeviceType.VirtualTarget'/>
      </div>
    </div>
  </div>
</template>

<style scoped>
.wrapper {
  display: flex;
  flex-direction: row;
  gap: 20px;
}

.mixer {
  background-color: #2d3230;
  padding: 15px;
  display: flex;
  flex-direction: row;
  gap: 25px;
}
</style>
