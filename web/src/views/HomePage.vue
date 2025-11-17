<script setup>

import {websocket} from "@/app/sockets.js";
import Router from "@/views/desktop/routing/Router.vue";
import Mixer from "@/views/Mixer.vue";
import ApplicationsList from "@/views/desktop/applications/ApplicationsList.vue";

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
    <div class="routing">
      <div class="applications">
        <ApplicationsList/>
      </div>
      <Router/>
    </div>
  </div>
</template>

<style scoped>
.content {
  position: absolute;
  inset: 0 0 0 0;

  display: flex;
  gap: 20px;
  flex-direction: column;
  align-items: stretch;

  overflow: hidden;
}

.mix_wrap {
  min-height: 250px;
  display: flex;
  flex: 1;
  flex-direction: row;
  gap: 20px;

  padding: 10px; /* Remove bottom padding */

  overflow-x: auto;
  border-bottom: 2px solid #3b413f;

  scrollbar-width: auto;
  scrollbar-color: #4a5150 #2a2e2d;
}

/*
  This is kinda awkward, chromium will place the scrollbar beneath the padding area, which is good
  but firefox places it *INSIDE* padding area, which results in content being occluded. So we'll
  check whether the scrollbar is a webkit or firefox one and adjust padding accordingly.

  Having to do this in 2025 is kinda sad.
 */
@supports not selector(::-webkit-scrollbar) {
  .mix_wrap {
    padding-bottom: 18px;
  }
}

.mix_wrap::-webkit-scrollbar {
  height: 12px;
}

.mix_wrap::-webkit-scrollbar-track {
  background: #2a2e2d;
  border-radius: 3px;
}

.mix_wrap::-webkit-scrollbar-thumb {
  background: #4a5150;
  border-radius: 3px;
}

.mix_wrap::-webkit-scrollbar-thumb:hover {
  background: #5a6160;
}

.routing {
  padding: 10px;
  display: flex;
  flex-direction: row;
}

.applications {
  min-width: 250px;
  /*max-width: 250px;*/
}


</style>
