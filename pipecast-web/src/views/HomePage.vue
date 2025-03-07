<script setup>

import {DeviceType, get_devices} from "@/pipecast/util.js";
import ChannelColumn from "@/components/channels/ChannelColumn.vue";
</script>

<template>
  <div class="wrapper">
    <div class="mixer">
      <div v-for="id in get_devices(DeviceType.PHYSICAL_SOURCE).keys()">
        <ChannelColumn :index=id :type='DeviceType.PHYSICAL_SOURCE'/>
      </div>
    </div>
    <div class="mixer">
      <div v-for="id in get_devices(DeviceType.VIRTUAL_SOURCE).keys()">
        <ChannelColumn :index=id :type='DeviceType.VIRTUAL_SOURCE'/>
      </div>
    </div>

    <div class="mixer">
      <div v-for="id in get_devices(DeviceType.PHYSICAL_TARGET).keys()">
        <ChannelColumn :index=id :type='DeviceType.PHYSICAL_TARGET'/>
      </div>
    </div>
    <div class="mixer">
      <div v-for="id in get_devices(DeviceType.VIRTUAL_TARGET).keys()">
        <ChannelColumn :index=id :type='DeviceType.VIRTUAL_TARGET'/>
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
