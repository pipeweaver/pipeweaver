<script>
import {store} from "@/app/store.js";
import {websocket} from "@/app/sockets.js";
import PopupBox from "@/views/desktop/inputs/PopupBox.vue";

export default {
  name: "Settings",
  components: {PopupBox},

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

    get_use_browser() {
      return store.getConfig().global_settings.use_browser;
    },
    set_use_browser(e) {
      websocket.send_daemon_command({"SetUseBrowser": e.target.checked})
    },

    restart_audio(e) {
      websocket.send_daemon_command("ResetAudio");
    },

    get_quantum_list() {
      return [
        {name: "Default", label: "Pipewire Configured"},
        {name: "Quantum8", label: "8"},
        {name: "Quantum16", label: "16"},
        {name: "Quantum32", label: "32"},
        {name: "Quantum64", label: "64"},
        {name: "Quantum128", label: "128"},
        {name: "Quantum256", label: "256"},
        {name: "Quantum512", label: "512"},
        {name: "Quantum768", label: "768"},
        {name: "Quantum1024", label: "1024"},
        {name: "Quantum1280", label: "1280"},
        {name: "Quantum1536", label: "1536"},
        {name: "Quantum1792", label: "1792"},
        {name: "Quantum2048", label: "2048"},
        {name: "Quantum2304", label: "2304"},
        {name: "Quantum2560", label: "2560"},
        {name: "Quantum2816", label: "2816"},
        {name: "Quantum3072", label: "3072"},
        {name: "Quantum3328", label: "3328"},
        {name: "Quantum3584", label: "3584"},
        {name: "Quantum3840", label: "3840"},
        {name: "Quantum4096", label: "4096"},
      ]
    },

    get_quantum_label(quantum) {
      console.log("Getting quantum label for " + quantum);

      for (let input of this.get_quantum_list()) {
        if (quantum === null) {
          return "Pipewire Configured";
        }

        if (quantum === input.name) {
          return input.label;
        }
      }
    },


    open_quantum_selector(e) {
      // The dialog only needs the target element to position itself, so we'll get the element
      // attaches to the event and pass that along instead of the raw event.
      const anchor = (e && e.currentTarget) ? e.currentTarget : (e && e.target) ? e.target : undefined;
      const event = Object.assign({}, e, {target: anchor});
      if (this.$refs.quantum_popup) {
        this.$refs.quantum_popup.showDialog(event, undefined, true, true);
      }
    },

    is_active(name) {
      if (name === "Default" && this.get_quantum() === null) {
        return true;
      }
      return this.get_quantum() === name;
    },

    get_quantum() {
      return store.getAudio().profile.audio_node_quantum
    },

    set_quantum(value) {
      this.$refs.quantum_popup.close();
      let quantum = this.get_quantum();

      console.log("Setting quantum to " + value);

      if (quantum === null && value !== "Default") {
        websocket.send_daemon_command({"SetAudioQuantum": value})
        return;
      }

      if (quantum !== null && value === "Default") {
        websocket.send_daemon_command({"SetAudioQuantum": null})
        return;
      }

      if (quantum === null && value === "Default") {
        return;
      }

      if (value !== quantum) {
        websocket.send_daemon_command({"SetAudioQuantum": value})
      }
    }
  }
}
</script>

<template>
  <PopupBox ref="quantum_popup">
    <div v-for="quantum in get_quantum_list()">
      <div
        :class="{ 'selected': is_active(quantum.name) }"
        class="entry"
        @click="set_quantum(quantum.name)">
        <span class="title">{{ quantum.label }}</span>
      </div>
    </div>
  </PopupBox>

  <div class="settings">
    <div class="flex-settings">
      <div style="margin-bottom: 10px">
        <input id="auto-start" :checked="get_autostart()" type="checkbox" @change="set_autostart"/>
        <label for="auto-start">Auto start</label>
      </div>
      <div>
        <input id="use-browser" :checked="get_use_browser()" type="checkbox"
               @change="set_use_browser"/>
        <label for="use-browser">Use Browser instead of App</label>
      </div>
    </div>
    <div>
      <div class="warning">
        <div style="text-align: center; font-weight: bold;">
          Restart Audio Engine
        </div>
        <div style="margin-bottom: 10px;">
          The following button will restart the audio engine, disconnecting and destroying all
          Pipeweaver nodes. After 2 seconds, it will attempt to recreate them again, this may break
          stuff.<br/><br/>
          Press this only if you're experiencing issues that cannot be resolved otherwise.<br/>
        </div>
        <div style="text-align: center">
          <button @click="restart_audio">Restart Audio Engine</button>
        </div>
      </div>
    </div>
    <div>
      <div class="warning">
        <div style="text-align: center; font-weight: bold;">Audio Buffer Size</div>
        <div style="margin-bottom: 10px;">
          The setting below adjusts the buffer size PipeWire uses when processing Pipeweaver
          audio.<br/><br/>

          Leave this as <b>“PipeWire Configured”</b> unless you need manual control and understand
          the impact on latency and CPU usage. Read more in the
          <a href="https://github.com/pipeweaver/pipeweaver/wiki/Audio-Buffer-Size" target="_blank">documentation</a>.

          <br/><br/>
          Changing this setting will restart the audio engine.
        </div>
        <div class="quantum">
          <div class="inner" @click="open_quantum_selector($event)">
            <span>{{ this.get_quantum_label(this.get_quantum()) }}</span>
            <font-awesome-icon :icon="['fas', 'angle-down']" class="selector-icon"/>
          </div>
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

.quantum {
  display: flex;
  align-items: center;
  justify-content: center;
}

.quantum .inner {
  display: flex;
  width: 100%;
  align-items: center;
  justify-content: space-between;
  border: 1px solid #666;
  box-sizing: border-box;
}

.quantum .inner:hover {
  background-color: #3b413f;
  cursor: pointer;
}

.quantum .inner span {
  white-space: nowrap;
  padding: 5px 5px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.quantum .inner svg {
  padding-right: 5px;
}

.selected {
  background-color: #214283;
}

.title {
  white-space: nowrap;
}

.entry {
  white-space: nowrap;
  padding: 4px 10px 4px 10px;
}

.entry:hover {
  background-color: #49514e;
  cursor: pointer;
}

.entry:not(:last-child) {
  border-bottom: 1px solid #3b413f;
}
</style>
