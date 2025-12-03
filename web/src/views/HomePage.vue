<script setup>

import {websocket} from "@/app/sockets.js";
import Router from "@/views/desktop/routing/Router.vue";
import Mixer from "@/views/Mixer.vue";
import ApplicationsList from "@/views/desktop/applications/ApplicationsList.vue";
import {nextTick, ref} from "vue";
import Settings from "@/views/Settings.vue";

const activeTab = ref('router');

function addDevice(type, e) {
  let name = prompt("Device Name:");

  // CreateNode(NodeType, String),
  let command = {
    "CreateNode": [type, name]
  }
  websocket.send_command(command)
}

async function switchTab(tab) {
  activeTab.value = tab;
  await nextTick();

  // Trigger a 'resize' event to make sure children are resized correctly
  window.dispatchEvent(new Event('resize'));
}

</script>

<template>
  <div class="content">
    <div class="mix_wrap">
      <Mixer :is_source="true"/>
      <Mixer :is_source="false"/>
    </div>
    <div class="routing">
      <div class="tabs">
        <button :class="['tab', { active: activeTab === 'router' }]" @click="switchTab('router')">
          Routing
        </button>
        <button :class="['tab', { active: activeTab === 'apps' }]" @click="switchTab('apps')">
          Applications
        </button>
        <button :class="['tab', { active: activeTab === 'settings' }]"
                @click="switchTab('settings')">
          <font-awesome-icon icon="fa-solid fa-gear"/>
        </button>
      </div>
      <div class="tab-content">
        <Router v-if="activeTab === 'router'"/>
        <ApplicationsList v-if="activeTab === 'apps'"/>
        <Settings v-if="activeTab === 'settings'"/>
      </div>
    </div>
  </div>
</template>

<style scoped>
.content {
  position: absolute;
  inset: 0 0 0 0;

  display: flex;
  flex-direction: column;
  align-items: stretch;

  overflow: hidden;
}

.mix_wrap {
  min-height: 250px;
  display: flex;
  flex: 1;
  flex-direction: row;

  gap: 15px;

  padding: 10px;
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
  display: flex;
  flex-direction: column;
}

.tabs {
  display: flex;
  gap: 2px;
  border-bottom: 2px solid #3b413f;
  margin-bottom: 10px;
  flex-shrink: 0;
}

.tab {
  padding: 10px 20px;
  background: #2a2e2d;
  border: none;
  color: #9ca3a0;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.2s ease;
}

.tab:hover {
  background: #353a39;
  color: #b8bfbc;
}

.tab.active {
  background: #4a5150;
  color: #ffffff;
}

.tab-content {
  flex: 1;
  overflow: auto;
  min-height: 0;
}
</style>
