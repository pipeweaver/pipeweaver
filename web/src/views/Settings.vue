<script>
import {store} from "@/app/store.js";
import {websocket} from "@/app/sockets.js";

export default {
  name: "Settings",

  data() {
    return {}
  },

  methods: {
    get_autostart() {
      return store.getConfig().auto_start;
    },
    set_autostart(e) {
      websocket.send_daemon_command({"SetAutoStart": e.target.checked})
    },

    restart_audio(e) {
      websocket.send_daemon_command("ResetAudio");
    }
  }
}
</script>

<template>
  <div class="settings">
    <div>
      <input id="auto-start" :checked="get_autostart()" type="checkbox" @change="set_autostart"/>
      <label for="auto-start">Auto start</label>
    </div>
    <div style="height: 40px"></div>
    <div>
      <button @click="restart_audio">Restart Audio Engine</button>
    </div>
  </div>
</template>

<style scoped>
.settings {
  padding: 10px;
  min-height: 200px;
  gap: 25px;
}
</style>
