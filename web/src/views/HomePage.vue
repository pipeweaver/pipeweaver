<script setup>

import {websocket} from "@/app/sockets.js";
import Router from "@/components/routing/Router.vue";
import Mixer from "@/views/Mixer.vue";

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
    <div class="mix_wrap">
      <Mixer :is_source="true"/>
      <Mixer :is_source="false"/>
    </div>
    <Router/>
  </div>
</template>

<style scoped>
.content {
  height: 100vh;
  display: flex;
  gap: 20px;
  flex-direction: column;
  align-items: stretch;
}

.mix_wrap {
  min-height: 250px;
  display: flex;
  flex: 1;
  flex-direction: row;
  gap: 20px;
}


</style>
