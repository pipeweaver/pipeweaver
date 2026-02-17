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
      console.log(target);
      let dev = get_device_by_id(target.id);
      if (dev.mix === "B") {
        return "#e07c24";
      }

      return "#59b1b6";
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
        <th v-for="source in getFullSourceList()" :key="source">{{ source.name }}</th>
      </tr>
      </thead>
      <tr v-for="(target, index) of getFullTargetList(true)" :key="target">

        <!-- Draw the 'Targets' Cell down the left on first iteration -->
        <th v-if="index === 0" :rowspan="getTargetCount()" class="rotated">
          <span>Targets</span>
        </th>

        <!-- Output the Channel Name -->
        <th>{{ target.name }}</th>

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
  background-color: #212624;
  margin: auto;
  width: fit-content;

  padding: 15px;
  display: flex;
  flex-direction: row;
  gap: 15px;
}


table {
  color: #fff;
  font-stretch: condensed;
  border-spacing: 4px;
  border-collapse: separate;
}

th {
  font-weight: normal;
  padding: 6px;
}

thead th:not(.subHeader) {
  background-color: #3b413f;
}

thead .subHeader th {
  background-color: #353937;
  min-width: 75px;
}

tr th {
  background-color: #353937;
}


.rotated {
  background-color: #3b413f;
  text-align: center;
  width: 15px;
}

.rotated span {
  writing-mode: vertical-rl;
  transform: rotate(180deg);
  white-space: nowrap;
}

.hidden {
  background-color: transparent !important;
}
</style>
