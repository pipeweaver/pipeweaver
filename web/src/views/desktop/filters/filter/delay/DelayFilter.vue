<script>
import {store} from "@/app/store.js";
import {websocket} from "@/app/sockets.js";
import DelayChannel from "@/views/desktop/filters/filter/delay/DelayChannel.vue";
import FlowLayout from "@/views/desktop/filters/layout/FlowLayout.vue";
import FlowItem from "@/views/desktop/filters/layout/FlowItem.vue";

export default {
  name: "DelayFilter",
  components: {FlowItem, FlowLayout, DelayChannel},
  props: {
    filterId: {type: String, required: true},
    filterType: {type: String, required: true}
  },

  methods: {
    getFilterConfig() {
      console.log(store.getAudio().filter_config[this.filterId]);
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

    get_values(is_left) {
      const config = this.getFilterConfig();
      console.log(config);
      if (!config) return {};
    }
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
  <div style="min-width: 600px; padding: 10px">
    <FlowLayout>
      <FlowItem width="200px">
        <DelayChannel :filterId="filterId" channel="l"/>
      </FlowItem>
      <FlowItem width="200px">
        <DelayChannel :filterId="filterId" channel="r"/>
      </FlowItem>
    </FlowLayout>
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
