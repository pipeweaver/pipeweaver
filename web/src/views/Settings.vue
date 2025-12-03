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
    <div class="flex-settings">
      <input id="auto-start" :checked="get_autostart()" type="checkbox" @change="set_autostart"/>
      <label for="auto-start">Auto start</label>
    </div>
    <div style="height: 20px"></div>
    <div>
      <div class="warning">
        <div style="text-align: center; font-weight: bold;">
          HERE BE DRAGONS
        </div>
        <div style="margin-bottom: 10px;">
          The following button will restart the audio engine, disconnecting and destroying all
          Pipeweaver nodes. After 2 seconds, it will attempt to recreate them again.<br/><br/>
          Press this only if you're experiencing issues that cannot be resolved otherwise.<br/>
        </div>
        <div style="text-align: center">
          <button @click="restart_audio">Restart Audio Engine</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings {
  padding: 10px;
  min-height: 200px;
  gap: 25px;
  display: flex;
}

.flex-settings {
  flex-grow: 1;
}

.warning {
  width: 600px;
  background-color: #370000;
  border: 1px solid #6e0000;
  color: #8e8e8e;
  padding: 6px;
}
</style>
