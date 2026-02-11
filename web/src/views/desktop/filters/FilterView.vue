<script>
import ModalOverlay from "@/views/desktop/components/ModalOverlay.vue";
import DelayFilter from "@/views/desktop/filters/filter/delay/DelayFilter.vue";
import GenericLV2 from "@/views/desktop/filters/filter/GenericLV2.vue";
import {get_device_by_id} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";
import {store} from "@/app/store.js";
import FilterListItem from "@/views/desktop/filters/FilterListItem.vue";

export default {
  name: "FilterView",
  components: {
    FilterListItem,
    ModalOverlay,
    DelayFilter,
    GenericLV2
  },
  props: {
    id: {type: String, required: true}
  },

  data() {
    return {
      activeFilter: undefined,
      // Map specific plugin URIs to components
      pluginComponents: {
        'http://lsp-plug.in/plugins/lv2/comp_delay_x2_stereo': 'DelayFilter',
      },
      // Fallback component for each filter type
      fallbackComponents: {
        'LV2': 'GenericLV2',
      }
    }
  },

  methods: {
    getFilterState(id) {
      if (store.getAudio().filter_config[id] === undefined) {
        console.error("Filter State Missing: " + id);
        return {state: "ERROR", message: null};
      }

      const state = store.getAudio().filter_config[id].state;

      if (state['FeatureMissing'] !== undefined) {
        return {state: "FeatureMissing", message: state['FeatureMissing']};
      }

      if (state['Error'] !== undefined) {
        return {state: "Error", message: state['Error']};
      }

      return {state: state, message: null};
    },

    show(e) {
      this.$refs.filterModal.openModal(undefined, undefined);
    },

    getName() {
      let device = get_device_by_id(this.id);
      return device.description.name;
    },

    addFilter(e) {
      let url = prompt("Enter Filter URL:");
      if (url === undefined || url === "" || url === null) {
        return;
      }

      console.log("Add Filter: " + url);

      let command = {
        "AddFilterToNode": [this.id, {
          LV2: {
            "plugin_uri": url,
            "values": {}
          }
        }]
      };
      websocket.send_command(command);
    },

    removeFilter(filter) {
      let id = this.getFilterInfo(filter).id;
      let command = {
        "RemoveFilter": id
      };
      websocket.send_command(command);
    },

    setActiveFilter(filter) {
      this.activeFilter = this.getFilterInfo(filter);
    },

    getFilters() {
      let device = get_device_by_id(this.id);
      return device.filters;
    },

    getFilterInfo(filter) {
      if (filter['LV2']) {
        return {
          id: filter['LV2'].id,
          type: 'LV2',
          identifier: filter['LV2'].plugin_uri,
        };
      }
    },

    getFilterName(filter) {
      let id = this.getFilterInfo(filter).id;
      if (store.getAudio().filter_config[id] === undefined) {
        return "Unknown Filter";
      }

      return store.getAudio().filter_config[id].name;
    },

    getFilterPageComponent(identifier, filterType) {
      // First try to find a specific component for this plugin URI
      if (this.pluginComponents[identifier]) {
        return this.pluginComponents[identifier];
      }

      // Fall back to generic component for this filter type
      return this.fallbackComponents[filterType] || null;
    }
  },

  computed: {
    currentFilterPageComponent() {
      if (!this.activeFilter) return null;
      return this.getFilterPageComponent(this.activeFilter.identifier, this.activeFilter.type);
    },

    filterState() {
      return this.getFilterState(this.activeFilter.id);
    },
  }
}
</script>

<template>
  <ModalOverlay body-padding="0" :show_footer="false" ref="filterModal" id="filterViewModal"
                title="Filters">
    <template v-slot:title>{{ getName() }} - We have EasyEffects at Home</template>
    <template v-slot:default>
      <div class="filter-wrapper">
        <div class="filter-list">
          <div class="add-filter" @click="addFilter">Add Filter</div>

          <FilterListItem
            v-for="filter in getFilters()"
            :key="getFilterInfo(filter).id"
            :filter="filter"
            :filter-info="getFilterInfo(filter)"
            :filter-name="getFilterName(filter)"
            @select="setActiveFilter"
            @remove="removeFilter"
          />
        </div>

        <div v-if="activeFilter === undefined" class="filter-page empty-state">
          <h3>No Filter Selected</h3>
          <p>Select a filter from the list or add a new one.</p>
          <p class="suggestion">Try:
            <code>http://lsp-plug.in/plugins/lv2/comp_delay_x2_stereo</code></p>
        </div>

        <div v-else>
          <div v-if="filterState.state === 'Running'">
            <component
              :is="currentFilterPageComponent"
              :filter-id="activeFilter.id"
              :filter-type="activeFilter.identifier"
            />
          </div>

          <!-- Error states -->
          <div v-else-if="filterState.state === 'NotFound'" class="error-state">
            This plugin was not found on your system.
          </div>
          <div v-else-if="filterState.state === 'NotCompatible'" class="error-state">
            This plugin is not compatible with Pipeweaver
          </div>
          <div v-else-if="filterState.state === 'FeatureMissing'" class="error-state">
            Feature not enabled: {{ filterState.message }}
          </div>
          <div v-else-if="filterState.state === 'Error'" class="error-state">
            Error: {{ filterState.message }}
          </div>
          <div v-else class="error-state">
            Internal Pipeweaver Error!
          </div>
        </div>
      </div>
    </template>
  </ModalOverlay>
</template>

<style scoped>
.filter-wrapper {
  display: flex;
  flex-direction: row;
  gap: 10px;
}

.filter-list {
  min-width: 300px;
  width: 300px;
  border-right: 1px solid #fff;
}

.filter-page {
  width: 800px;
}

.empty-state, .error-state {
  padding: 40px;
  text-align: center;
  color: #888;
}

.suggestion {
  margin-top: 20px;
  padding: 10px;
  background-color: #2a2f2e;
  border-radius: 4px;
}

.suggestion code {
  color: #6bb6ff;
  font-size: 0.9em;
}

.add-filter {
  margin: 0 5px 5px 5px;
  padding: 10px;
  border-bottom: 1px solid #fff;
  cursor: pointer;
  text-align: center;
}

.add-filter:hover {
  background-color: #353a39;
}
</style>
