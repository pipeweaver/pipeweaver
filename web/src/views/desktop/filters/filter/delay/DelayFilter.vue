<script>
import {store} from "@/app/store.js";
import {websocket} from "@/app/sockets.js";
import DelayChannel from "@/views/desktop/filters/filter/delay/DelayChannel.vue";

export default {
  name: "DelayFilter",
  components: {DelayChannel},
  props: {
    filterId: {type: String, required: true},
    filterType: {type: String, required: true}
  },

  methods: {
    getFilterConfig() {
      return store.getAudio().filter_config[this.filterId];
    },

    setParameterValue(paramId, value) {
      let send_value;

      if (typeof value === 'boolean') {
        send_value = {"Bool": value};
      } else if (Number.isInteger(value)) {
        send_value = {"Int32": value};
      } else {
        send_value = {"Float32": value};
      }

      let command = {
        "SetFilterValue": [this.filterId, paramId, send_value]
      };
      websocket.send_command(command);
    },
  },

  computed: {
    filterState() {
      return this.getFilterState(this.filterId);
    },

    delayParameters() {
      const config = this.getFilterConfig();
      if (!config) return [];
      return config.parameters.filter(p => p.is_input !== false);
    }
  }
}
</script>

<template>
  <div class="delay-filter-page">
    <span>Aww Shit, a delay filter :D</span>
    <div class="plugin-uri">{{ filterType }}</div>
    <div class="delay-controls">
      <DelayChannel channel="l"/>
      <DelayChannel channel="r"/>
    </div>
  </div>
</template>

<style scoped>
.delay-filter-page {
  width: 800px;
  padding: 20px;
}

.plugin-uri {
  font-size: 0.8em;
  color: #888;
  margin-bottom: 20px;
}

.delay-controls {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.delay-section h4 {
  margin-bottom: 15px;
  color: #fff;
}

.error-state a {
  color: #6bb6ff;
}

select {
  padding: 5px 10px;
  background-color: #1a1f1e;
  border: 1px solid #444;
  color: #fff;
  border-radius: 3px;
  min-width: 150px;
}
</style>
