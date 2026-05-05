<script>

import {
  DeviceType,
  get_device_by_id,
  get_devices,
  getFullSourceList,
  getFullTargetList
} from "@/app/util.js";
import RoutingCell from "@/views/desktop/routing/RoutingCell.vue";
import {store} from "@/app/store.js";
import {websocket} from "@/app/sockets.js";

export default {
  name: 'Router',
  components: {RoutingCell},

  methods: {
    getFullSourceList,
    getFullTargetList,
    getSourceCount: function () {
      return get_devices(DeviceType.PhysicalSource).length + get_devices(DeviceType.VirtualSource).length;
    },

    getTargetCount: function () {
      return get_devices(DeviceType.PhysicalTarget).length + get_devices(DeviceType.VirtualTarget).length;
    },

    isEnabled: function (source, target) {
      if (store.getProfile().routes === undefined) {
        return false
      }

      if (store.getProfile().routes[source] === undefined) {
        return false
      }

      return store.getProfile().routes[source].includes(target);
    },

    getCheckColour: function (target) {
      let dev = get_device_by_id(target.id);
      if (dev.mix === "B") {
        return "#e07c24";
      }

      return "#59b1b6";
    },

    getColour: function (target) {
      let dev = get_device_by_id(target.id);
      let colour = dev.description.colour;
      let base = `rgba(${colour.red}, ${colour.green}, ${colour.blue}, 0.6)`

      return base;
    },

    handleClick: function (source, target) {
      // SetRoute(Ulid, Ulid, bool)
      let enabled = !this.isEnabled(source, target);
      let command = {
        "SetRoute": [source, target, enabled]
      };
      websocket.send_command(command);
    }
  }
}
</script>

<template>
  <div class="routing">
    <table>
      <thead>
      <tr>
        <th class="hidden" colspan="2">&nbsp;</th>
        <th :colspan="getSourceCount()">Sources</th>
      </tr>
      <tr class="subHeader">
        <th class="hidden" colspan="2">&nbsp;</th>
        <th v-for="source in getFullSourceList()" :key="source"
            :style="{
              background: `linear-gradient(to top, ${getColour(source)}, #131A22)`
            }">
          {{ source.name }}
        </th>
      </tr>
      </thead>
      <tr v-for="(target, index) of getFullTargetList(true)" :key="target">

        <!-- Draw the 'Targets' Cell down the left on first iteration -->
        <th v-if="index === 0" :rowspan="getTargetCount()" class="rotated">
          <span>Targets</span>
        </th>

        <!-- Output the Channel Name -->
        <th :style="{
          background: `linear-gradient(to left, ${getColour(target)}, #131A22)`

  }">{{ target.name }}
        </th>

        <!-- Output the Source cells for this Target -->
        <RoutingCell v-for="source in getFullSourceList()" :key="source"
                     :colour="getCheckColour(target)"
                     :enabled="isEnabled(source.id, target.id)" :source="source.id"
                     :target="target.id" @clicked="handleClick"/>
      </tr>
    </table>
  </div>
</template>

<style scoped>
.routing {
  margin: auto;
  width: fit-content;

  padding: 15px;
  display: flex;
  flex-direction: row;
}


table {
  border-spacing: 0;
}

th {
  font-weight: normal;
  padding: 10px;
}

thead tr:not(.subHeader) th {
  text-transform: uppercase;
  color: var(--cyan);
  text-shadow: 0 0 4px var(--cyan);
  letter-spacing: 0.20em;
  background-color: transparent;
  font-weight: bold;
  border: none !important;
}

thead .subHeader th {
  min-width: 75px;
  font-weight: bold;
}

thead tr.subHeader th:nth-child(2) {
  border-top-left-radius: 8px;
}

thead tr.subHeader th:last-child {
  border-top-right-radius: 8px;
}

tr th:nth-child(2) {
  border-top-left-radius: 8px;
}

tr:last-child th:nth-child(1) {
  border-bottom-left-radius: 8px;
}

tr {
  /* We set this so cells can be 100% height */
  height: 1px;
}

tr th {
  border: var(--border);
  font-weight: bold;
}

.rotated {
  text-transform: uppercase;
  color: var(--cyan);
  text-shadow: 0 0 4px var(--cyan);
  letter-spacing: 0.20em;
  background-color: transparent;
  font-weight: bold;
  width: 15px;
  border: none !important;
}

.rotated span {
  writing-mode: vertical-rl;
  transform: rotate(180deg);
  white-space: nowrap;
}

.hidden {
  background: transparent none !important;
  border: none !important;
}
</style>
