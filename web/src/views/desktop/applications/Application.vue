<script>
import {store} from "@/app/store.js";
import {DeviceType, get_devices} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";
import ApplicationNodes from "@/views/desktop/applications/ApplicationNodes.vue";

export default {
  name: "Application",
  components: {ApplicationNodes},

  props: {
    isSource: {type: Boolean, required: true},
    processName: {type: String, required: true},
  },

  methods: {
    get_source_key() {
      return this.isSource ? "Source" : "Target";
    },

    get_application_list() {
      return Object.keys(store.getApplications()[this.get_source_key()][this.processName]);
    },

    get_application_target(app) {
      let process = store.getProfile().application_mapping[this.get_source_key()][this.processName];
      if (process && process[app]) {
        // Do we have a child node for this app?
        return process[app];
      } else {
        return "-1";
      }
    },

    get_application_nodes(app) {
      return store.getApplications()[this.get_source_key()][this.processName][app];
    },

    get_target_devices() {
      return get_devices(DeviceType.VirtualTarget)
    },
    get_source_devices() {
      return get_devices(DeviceType.VirtualSource)
    },

    get_devices() {
      return this.isSource ? this.get_source_devices() : this.get_target_devices();
    },

    set_application_target(e, app) {
      // SetApplicationRoute(AppDefinition, Ulid),
      // AppDefinition {
      //   pub device_type: DeviceType,
      //     pub process: String,
      //     pub name: String,
      // }

      let definition = {
        device_type: this.get_source_key(),
        process: this.processName,
        name: app
      };

      let command = {
        "SetApplicationRoute": [definition, e.target.value]
      }
      if (e.target.value === "-1") {
        command = {
          "ClearApplicationRoute": definition
        }
      }
      websocket.send_command(command);
    },

    on_app_click(e, index) {
      // Try and locate the button pressed.
      let found = false;
      let element = e.target;
      if (element.nodeName.toLowerCase() === "button") {
        element.firstChild.style.transform = "rotate(-90deg)";
      } else {
        while (!found) {
          if (element.nodeName === "svg" || element.nodeName === "path") {
            element = element.parentNode;
            continue;
          }
          found = true;
        }
        element.style.transform = "rotate(-90deg)";
      }
      console.log(this.$refs.nodes);
      this.$refs.nodes[index].show(e);
    },

    on_app_close(index) {
      this.$refs.settings_icon[index].style.transform = "";
    }
  },


}
</script>

<template>


  <div v-for="(app, index) in get_application_list()">
    <ApplicationNodes ref="nodes" :is-source="this.isSource" :nodes="get_application_nodes(app)"
                      @closed="on_app_close(index)"/>
    <div class="app">
      <div class="name">{{ app }}</div>
      <div class="selector">
        <select @change="e => set_application_target(e, app)">
          <option :selected="this.get_application_target(app) === -1" value="-1">Default</option>
          <option v-for="device in get_devices()"
                  :selected="this.get_application_target(app) === device.description.id"
                  :value="device.description.id">
            {{ device.description.name }}
          </option>
        </select>
      </div>
      <div class="settings">
        <button ref="app_icon" @click="e => on_app_click(e, index)">
          <span ref="settings_icon" class="rotate">
            <font-awesome-icon :icon="['fas', 'angle-down']"/>
          </span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: row;
}

.app .name {
  padding: 5px 5px 5px 15px;
  flex-grow: 1;
  font-weight: bold;
}

.app .selector {
  padding: 5px;
}

.app .selector select {
  border: 0;
  padding: 0 5px;
}


.app .settings button {
  all: unset;
  padding: 5px;
}

.app .settings button span {
  display: inline-block;
  padding: 5px;
}

.app .settings button span.rotate {
  padding: 0;
  margin: 0;
  transition: transform 0.2s ease;
}

</style>
