<script setup>

import {DeviceType, get_devices} from "@/app/util.js";
import ChannelColumn from "@/components/channels/ChannelColumn.vue";
import {websocket} from "@/app/sockets.js";
import Router from "@/components/routing/Router.vue";

function addDevice(type, e) {
  let name = prompt("Device Name:");

  // CreateNode(NodeType, String),
  let command = {
    "CreateNode": [type, name]
  }
  websocket.send_command(command)
}

</script>

<template>
  <div class="content">
    <div class="wrapper">
      <div class="mixer">
        <div v-for="id in get_devices(DeviceType.PhysicalSource).keys()">
          <ChannelColumn :index=id :type='DeviceType.PhysicalSource'/>
        </div>
        <button style="font-size: 40px" @click="e => addDevice(DeviceType.PhysicalSource, e)">+
        </button>
      </div>
      <div class="mixer">
        <div v-for="id in get_devices(DeviceType.VirtualSource).keys()">
          <ChannelColumn :index=id :type='DeviceType.VirtualSource'/>
        </div>
        <button style="font-size: 40px" @click="e => addDevice(DeviceType.VirtualSource, e)">+
        </button>
      </div>
      <div class="mixer">
        <div v-for="id in get_devices(DeviceType.PhysicalTarget).keys()">
          <ChannelColumn :index=id :type='DeviceType.PhysicalTarget'/>
        </div>
        <button style="font-size: 40px" @click="e => addDevice(DeviceType.PhysicalTarget, e)">+
        </button>
      </div>
      <div class="mixer">
        <div v-for="id in get_devices(DeviceType.VirtualTarget).keys()">
          <ChannelColumn :index=id :type='DeviceType.VirtualTarget'/>
        </div>
        <button style="font-size: 40px" @click="e => addDevice(DeviceType.VirtualTarget, e)">+
        </button>
      </div>
    </div>
    <div>
      <Router/>
    </div>
  </div>
</template>

<style scoped>
.content {
  height: 100vh;
  display: flex;
  gap: 20px;
  flex-direction: column;
  align-items: stretch;;
}

.wrapper {
  min-height: 250px;
  display: flex;
  flex: 1;
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

.mixer button {
  color: #fff;
  border: 1px solid #666666;
  background-color: #353937;
  border-radius: 5px;
}

.mixer button:hover {
  background-color: #49514e;
  cursor: pointer;
}
</style>
