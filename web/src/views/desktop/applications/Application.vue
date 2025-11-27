<script>
import {store} from "@/app/store.js";
import {DeviceType, get_devices} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";
import ApplicationNodes from "@/views/desktop/applications/ApplicationNodes.vue";
import PopupBox from "@/views/desktop/inputs/PopupBox.vue";

export default {
  name: "Application",
  components: {PopupBox, ApplicationNodes},

  emits: ['request-remove'],

  props: {
    isSource: {type: Boolean, required: true},
    processName: {type: String, required: true},
    apps: {type: Array, default: () => []}
  },

  data() {
    return {
      removalTimeout: 10000,
      criticalTimer: 3000,

      pendingTimers: {},
      _checkerInterval: null,
      _now: Date.now(),

      circleRadius: 10,
      circleStroke: 3,
      circleViewSize: 28
    }
  },

  methods: {
    get_source_key() {
      return this.isSource ? "Source" : "Target";
    },

    get_application_list() {
      return this.apps || [];
    },

    get_application_target(app) {
      const profile = store.getProfile && store.getProfile();
      if (!profile || !profile.application_mapping) return "-1";
      const mappingRoot = profile.application_mapping[this.get_source_key()] || {};
      const process = mappingRoot[this.processName] || {};
      return process[app] !== undefined ? String(process[app]) : "-1";
    },

    get_application_target_name(app) {
      let target = this.get_application_target(app);
      if (target === "-1") {
        return "Default";
      }
      const devices = this.get_devices();
      for (const device of devices) {
        if (String(device.description.id) === target) {
          return device.description.name;
        }
      }
    },

    get_application_nodes(app) {
      const appsRoot = store.getApplications && store.getApplications()[this.get_source_key()];
      const proc = (appsRoot && appsRoot[this.processName]) || {};
      const nodes = proc ? proc[app] : undefined;
      return Array.isArray(nodes) ? nodes : [];
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

    set_application_target(app, index, value) {
      this.$refs['popups'][index].close();

      let definition = {
        device_type: this.get_source_key(),
        process: this.processName,
        name: app
      };

      let command = {
        "SetApplicationRoute": [definition, value]
      }
      if (value === "-1") {
        command = {
          "ClearApplicationRoute": definition
        }
      }
      websocket.send_command(command);
    },

    on_app_click(e, index) {
      let found = false;
      let element = e.target;
      if (element.nodeName.toLowerCase() === "button") {
        if (element.firstChild && element.firstChild.style) {
          element.firstChild.style.transform = "rotate(-90deg)";
        }
      } else {
        while (!found) {
          if (element.nodeName === "svg" || element.nodeName === "path") {
            element = element.parentNode;
            continue;
          }
          found = true;
        }
        if (element && element.style) element.style.transform = "rotate(-90deg)";
      }
      if (this.$refs.nodes && this.$refs.nodes[index]) {
        this.$refs.nodes[index].show(e);
      }
    },

    on_app_close(index) {
      if (this.$refs.settings_icon && this.$refs.settings_icon[index]) {
        this.$refs.settings_icon[index].style.transform = "";
      }
    },

    start_pending_removal(app) {
      if (this.pendingTimers[app]) return;

      // Close the ApplicationNode if it's open
      const index = this.get_application_list().indexOf(app);
      if (index !== -1 && this.$refs.nodes && this.$refs.nodes[index]) {
        this.$refs.nodes[index].close();
      }

      const initialDuration = this.removalTimeout;
      const start = Date.now();
      const id = setTimeout(() => {
        if (this.$delete) {
          this.$delete(this.pendingTimers, app);
        } else {
          delete this.pendingTimers[app];
        }

        // Send up upstream trigger to remove this app
        this.$emit('request-remove', app);
      }, initialDuration);

      const entry = {id, start, initialDuration, elapsed: 0, paused: false};
      if (this.$set) {
        this.$set(this.pendingTimers, app, entry);
      } else {
        this.pendingTimers[app] = entry;
      }
    },

    pause_pending_removal(app) {
      const e = this.pendingTimers[app];
      if (!e || e.paused) return;
      const now = Date.now();
      const elapsedSoFar = (e.elapsed || 0) + Math.max(0, now - e.start);
      const total = e.initialDuration || this.removalTimeout;
      const remaining = Math.max(0, total - elapsedSoFar);
      if (e.id) clearTimeout(e.id);
      const newEntry = Object.assign({}, e, {
        id: null,
        paused: true,
        elapsed: elapsedSoFar,
        remaining
      });
      if (this.$set) {
        this.$set(this.pendingTimers, app, newEntry);
      } else {
        this.pendingTimers[app] = newEntry;
      }
    },

    resume_pending_removal(app) {
      const e = this.pendingTimers[app];
      if (!e || !e.paused) return;
      const total = e.initialDuration || this.removalTimeout;
      const remaining = Math.max(0, total - (e.elapsed || 0));
      if (remaining <= 0) {
        if (this.$delete) {
          this.$delete(this.pendingTimers, app);
        } else {
          delete this.pendingTimers[app];
        }
        this.$emit('request-remove', app);
        return;
      }

      const start = Date.now();
      const id = setTimeout(() => {
        if (this.$delete) {
          this.$delete(this.pendingTimers, app);
        } else {
          delete this.pendingTimers[app];
        }
        this.$emit('request-remove', app);
      }, remaining);

      const newEntry = {
        id,
        start,
        initialDuration: e.initialDuration || this.removalTimeout,
        elapsed: e.elapsed || 0,
        paused: false
      };
      if (this.$set) {
        this.$set(this.pendingTimers, app, newEntry);
      } else {
        this.pendingTimers[app] = newEntry;
      }
    },

    stop_pending_removal(app) {
      const entry = this.pendingTimers[app];
      if (entry && entry.id) {
        clearTimeout(entry.id);
        if (this.$delete) {
          this.$delete(this.pendingTimers, app);
        } else {
          delete this.pendingTimers[app];
        }
      } else if (entry && entry.paused) {
        if (this.$delete) {
          this.$delete(this.pendingTimers, app);
        } else {
          delete this.pendingTimers[app];
        }
      }
    },

    check_apps() {
      this._now = Date.now();

      for (const app of this.get_application_list()) {
        const nodes = this.get_application_nodes(app);
        const hasNodes = Array.isArray(nodes) && nodes.length > 0;
        if (!hasNodes) {
          this.start_pending_removal(app);
        } else {
          this.stop_pending_removal(app);
        }
      }

      for (const app of Object.keys({...this.pendingTimers})) {
        if (!this.get_application_list().includes(app)) {
          const entry = this.pendingTimers[app];
          if (entry && entry.id) clearTimeout(entry.id);
          if (this.$delete) {
            this.$delete(this.pendingTimers, app);
          } else {
            delete this.pendingTimers[app];
          }
        }
      }
    },

    get_removal_progress(app) {
      const e = this.pendingTimers[app];
      if (!e) return 0;
      const total = e.initialDuration || this.removalTimeout;
      if (total <= 0) return 1;
      let elapsed = e.elapsed || 0;
      if (!e.paused) elapsed += Math.max(0, this._now - e.start);
      return Math.min(1, elapsed / total);
    },

    get_remaining_ms(app) {
      const e = this.pendingTimers[app];
      if (!e) return 0;
      const total = e.initialDuration || this.removalTimeout;
      const elapsed = e.paused ? (e.elapsed || 0) : ((e.elapsed || 0) + Math.max(0, this._now - e.start));
      return Math.max(0, total - elapsed);
    },

    get_circle_circumference() {
      return 2 * Math.PI * this.circleRadius;
    },

    get_circle_offset(app) {
      const circumference = this.get_circle_circumference();
      const progress = this.get_removal_progress(app);
      return circumference * (1 - progress);
    },

    get_circle_color(app) {
      const e = this.pendingTimers[app];
      if (!e) return "#ffffff";
      const remaining = this.get_remaining_ms(app);
      return remaining > this.criticalTimer ? "#ffffff" : "#d9534f";
    },

    hasPending(app) {
      return !!(this.pendingTimers && this.pendingTimers[app]);
    },

    open_selector(e, app, id) {
      this.pause_pending_removal(app);

      // The dialog only needs the target element to position itself, so we'll get the element
      // attaches to the event and pass that along instead of the raw event.
      const anchor = (e && e.currentTarget) ? e.currentTarget : (e && e.target) ? e.target : undefined;
      const event = Object.assign({}, e, {target: anchor});
      if (this.$refs.popups && this.$refs.popups[id]) {
        // pass the anchor element (instead of the raw target)
        this.$refs.popups[id].showDialog(event, app, undefined, true);
      }
    },
  },

  mounted() {
    this._checkerInterval = setInterval(() => {
      this.check_apps();
    }, 100);
    this.check_apps();
  },

  beforeUnmount() {
    if (this._checkerInterval) {
      clearInterval(this._checkerInterval);
      this._checkerInterval = null;
    }
    for (const entry of Object.values(this.pendingTimers)) {
      if (entry && entry.id) clearTimeout(entry.id);
    }
    this.pendingTimers = {};
  },

  computed: {
    maxDeviceNameWidth() {
      const devices = this.get_devices();
      if (!devices.length) return '120px';

      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      // Match the actual font used in .inner
      ctx.font = '14px sans-serif'; // Adjust based on your actual font-size and font-family

      let maxWidth = 0;
      devices.forEach(device => {
        const width = ctx.measureText(device.description.name).width;
        maxWidth = Math.max(maxWidth, width);
      });

      if (maxWidth + 10 > 95) {
        return '95px';
      }

      // Reduce padding - adjust the value based on icon width
      return maxWidth + 'px';
    }
  }
}
</script>

<template>
  <div v-for="(app, index) in get_application_list()" :key="app">
    <ApplicationNodes ref="nodes" :is-source="this.isSource" :nodes="get_application_nodes(app)"
                      @closed="on_app_close(index)"/>

    <PopupBox ref="popups" @closed="resume_pending_removal(app)">
      <div :class="{ 'selected': this.get_application_target(app) === '-1' }"
           :style="{ 'min-width': `calc(${maxDeviceNameWidth} + 3px)` }" class="entry"
           @click="set_application_target(app, index, '-1')">
        <span class="title">Default</span>
      </div>

      <div v-for="device in get_devices()">
        <div
          :class="{ 'selected': String(this.get_application_target(app)) === String(device.description.id) }"
          :style="{ 'min-width': `calc(${maxDeviceNameWidth} + 3px )` }"
          class="entry"
          @click="set_application_target(app, index, device.description.id)">
          <span class="title">{{ device.description.name }}</span>
        </div>
      </div>
    </PopupBox>

    <div class="app">
      <div class="name">{{ app }}</div>
      <div :style="{ width: `calc(${maxDeviceNameWidth} + 25px)` }" class="selector">
        <div class="inner" @click="open_selector($event, app, index)">
          <span v-if="this.get_application_target(app) === '-1'">Default</span>
          <span v-else>{{ this.get_application_target_name(app) }}</span>
          <font-awesome-icon :icon="['fas', 'angle-down']" class="selector-icon"/>
        </div>
      </div>
      <div class="settings">
        <button v-if="!hasPending(app)" ref="app_icon" :title="'Open'" class="settings-btn"
                @click="e => on_app_click(e, index)">
          <span ref="settings_icon" class="rotate">
            <font-awesome-icon :icon="['fas', 'angle-down']"/>
          </span>
        </button>
        <div v-else class="settings-placeholder">
          <svg :height="circleViewSize" :viewBox="`0 0 ${circleViewSize} ${circleViewSize}`"
               :width="circleViewSize"
               aria-hidden="true"
               class="countdown" focusable="false" xmlns="http://www.w3.org/2000/svg">
            <g :transform="`translate(${circleViewSize/2},${circleViewSize/2})`">
              <circle :r="circleRadius" :stroke-width="circleStroke" class="bg" fill="none"
                      stroke="#eee"/>
              <circle :r="circleRadius" :stroke="get_circle_color(app)"
                      :stroke-dasharray="get_circle_circumference()"
                      :stroke-dashoffset="get_circle_offset(app)"
                      :stroke-width="circleStroke"
                      class="progress"
                      fill="none"
                      stroke-linecap="round"
                      transform="rotate(-90)"/>
            </g>
          </svg>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: row;

  align-items: center;
  justify-content: center;
}

.app .name {
  padding: 5px 5px 5px 15px;
  flex-grow: 1;
  font-weight: bold;
}

.app .selector {
  display: flex;
  align-items: center;
  justify-content: center;
}

.app .selector .inner {
  display: flex;
  width: 100%;
  align-items: center;
  justify-content: space-between;
  border: 1px solid #666;
  box-sizing: border-box;
}

.app .selector .inner:hover {
  background-color: #3b413f;
  cursor: pointer;
}

.app .selector .inner span {
  padding: 2px 5px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.app .selector .inner svg {
  padding-right: 5px;
}


.app .settings {
  display: flex;
  align-items: center;
  justify-content: center;
}

.app .settings button {
  all: unset;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border-radius: 4px;
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

.app .settings-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border-radius: 4px;
}

.countdown {
  display: block;
  width: 100%;
  height: 100%;
}

.countdown .bg {
  opacity: 0.4;
}

.countdown .progress {
  transition: stroke-dashoffset 0.1s linear;
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
