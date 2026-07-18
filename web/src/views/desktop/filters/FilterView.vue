<script>
import ModalOverlay from "@/views/desktop/components/ModalOverlay.vue";
import DelayFilter from "@/views/desktop/filters/filter/delay/DelayFilter.vue";
import CompressorFilter from "@/views/desktop/filters/filter/compressor/CompressorFilter.vue";
import GateFilter from "@/views/desktop/filters/filter/gate/GateFilter.vue";
import ExpanderFilter from "@/views/desktop/filters/filter/expander/ExpanderFilter.vue";
import LimiterFilter from "@/views/desktop/filters/filter/limiter/LimiterFilter.vue";
import EqualiserFilter from "@/views/desktop/filters/filter/equaliser/EqualiserFilter.vue";
import GenericLV2 from "@/views/desktop/filters/filter/GenericLV2.vue";
import {get_device_by_id} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";
import {store} from "@/app/store.js";
import FilterListItem from "@/views/desktop/filters/FilterListItem.vue";
import {Sortable} from "@shopify/draggable";
import AddFilterModal from "@/views/desktop/filters/AddFilterModal.vue";
import MultibandCompressorFilter
  from "@/views/desktop/filters/filter/multiband_compressor/MultibandCompressorFilter.vue";
import MultibandGateFilter
  from "@/views/desktop/filters/filter/multiband_gate/MultibandGateFilter.vue";
import BassEnhancerFilter
  from "@/views/desktop/filters/filter/bass_enhancer/BassEnhancerFilter.vue";
import CrusherFilter from "@/views/desktop/filters/filter/crusher/CrusherFilter.vue";
import DeesserFilter from "@/views/desktop/filters/filter/deesser/DeesserFilter.vue";
import ExciterFilter from "@/views/desktop/filters/filter/exciter/ExciterFilter.vue";
import StereoToolsFilter from "@/views/desktop/filters/filter/stereo_tools/StereoToolsFilter.vue";
import FilterFilter from "@/views/desktop/filters/filter/filter/FilterFilter.vue";
import LoudnessFilter from "@/views/desktop/filters/filter/loudness/LoudnessFilter.vue";
import MaximizerFilter from "@/views/desktop/filters/filter/maximizer/MaximizerFilter.vue";
import ReverberationFilter
  from "@/views/desktop/filters/filter/reverberation/ReverberationFilter.vue";

const INTERNAL_SCALE = 0.8;

export default {
  name: "FilterView",
  components: {
    AddFilterModal,
    FilterListItem,
    ModalOverlay,
    DelayFilter,
    CompressorFilter,
    GateFilter,
    ExpanderFilter,
    LimiterFilter,
    EqualiserFilter,
    MultibandCompressorFilter,
    MultibandGateFilter,
    BassEnhancerFilter,
    CrusherFilter,
    DeesserFilter,
    ExciterFilter,
    StereoToolsFilter,
    FilterFilter,
    LoudnessFilter,
    MaximizerFilter,
    ReverberationFilter,
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
        "http://lsp-plug.in/plugins/lv2/comp_delay_x2_stereo": {
          display: "Delay",
          component: "DelayFilter",
        },
        "http://lsp-plug.in/plugins/lv2/compressor_stereo": {
          display: "Compressor",
          component: "CompressorFilter",
        },
        "http://lsp-plug.in/plugins/lv2/gate_stereo": {
          display: "Gate",
          component: "GateFilter",
        },
        "http://lsp-plug.in/plugins/lv2/expander_stereo": {
          display: "Expander",
          component: "ExpanderFilter",
        },
        "http://lsp-plug.in/plugins/lv2/limiter_stereo": {
          display: "Limiter",
          component: "LimiterFilter",
        },
        "http://lsp-plug.in/plugins/lv2/para_equalizer_x32_lr": {
          display: "Equaliser",
          component: "EqualiserFilter",
        },
        "http://lsp-plug.in/plugins/lv2/mb_compressor_stereo": {
          display: "Multiband Compressor",
          component: "MultibandCompressorFilter",
        },
        "http://lsp-plug.in/plugins/lv2/mb_gate_stereo": {
          display: "Multiband Gate",
          component: "MultibandGateFilter",
        },
        "http://calf.sourceforge.net/plugins/BassEnhancer": {
          display: "Bass Enhancer",
          component: "BassEnhancerFilter",
        },
        "http://calf.sourceforge.net/plugins/Crusher": {
          display: "Crusher",
          component: "CrusherFilter",
        },
        "http://calf.sourceforge.net/plugins/Deesser": {
          display: "Deesser",
          component: "DeesserFilter",
        },
        "http://calf.sourceforge.net/plugins/Exciter": {
          display: "Exciter",
          component: "ExciterFilter",
        },
        "http://calf.sourceforge.net/plugins/StereoTools": {
          display: "Stereo Tools",
          component: "StereoToolsFilter",
        },
        "http://lsp-plug.in/plugins/lv2/filter_stereo": {
          display: "Filter",
          component: "FilterFilter",
        },
        "http://lsp-plug.in/plugins/lv2/loud_comp_stereo": {
          display: "Loudness",
          component: "LoudnessFilter",
        },
        "urn:zamaudio:ZaMaximX2": {
          display: "Maximizer",
          component: "MaximizerFilter",
        },
        "http://calf.sourceforge.net/plugins/Reverb": {
          display: "Reverberation",
          component: "ReverberationFilter",
        },
      },
      // Fallback component for each filter type
      fallbackComponents: {
        'LV2': 'GenericLV2',
      }
    }
  },

  methods: {
    setup_draggable() {
      if (this.draggable) {
        this.draggable.destroy();
        this.draggable = null;
      }

      const container = this.$refs.filters;
      const draggable = new Sortable(container, {
        draggable: '.filter-item',
        handle: '.filter-drag-handle',
        delay: 0,

        mirror: {
          xAxis: false,
          yAxis: false,
          constrainDimensions: true,
        },
      });

      draggable.on('drag:start', (event) => {
        event.source.dataset.originalLeft = event.source.getBoundingClientRect().left.toString();
        this.draggedId = event.source.dataset.id;
      })

      draggable.on('mirror:created', (event) => {
        event.source.classList.add('drag-placeholder');
        event.mirror.classList.add('custom-mirror');
        document.body.classList.add('dragging');
      });

      draggable.on('drag:move', (event) => {
        const mirror = document.querySelector('.draggable-mirror');
        const placeholder = document.querySelector('.drag-placeholder');

        if (!mirror || !placeholder) return;
        let positionX = placeholder.getBoundingClientRect().left;
        let positionY = placeholder.getBoundingClientRect().top;

        mirror.style.transform = `translate3d(${positionX}px, ${positionY}px, 0px) scale(${INTERNAL_SCALE})`;
        if (!mirror.classList.contains("custom-mirror-small")) {
          mirror.classList.add("custom-mirror-small");
        }
      });

      draggable.on('drag:stop', (event) => {
        const mirror = document.querySelector('.draggable-mirror');
        const dev_id = event.source.dataset.id;
        if (mirror) {
          // Clone the mirror to keep visible after drag ends
          const clone = mirror.cloneNode(true);
          clone.style.cssText = mirror.style.cssText;

          // Do not let Shopify Draggable clean up our restore clone.
          clone.classList.remove('draggable-mirror');
          clone.classList.remove('drag-placeholder');
          clone.classList.add('restore-mirror');

          const appRoot = document.getElementById('app');
          appRoot.appendChild(clone);

          const placeholder = document.querySelector('.drag-placeholder');
          if (!placeholder) return;

          let localX = event.source.getBoundingClientRect().left;
          let localY = event.source.getBoundingClientRect().top;

          // Animate back on the next frame so the browser sees a real transition.
          requestAnimationFrame(() => {
            clone.style.transform = `translate3d(${localX}px, ${localY}px, 0px) scale(1)`;
          });

          // Remove the cursor drag icon
          document.body.classList.remove('dragging');

          console.log("Dragged: " + dev_id);

          // Forcibly re-add the placeholder class to the target location
          let ref = this.$refs[dev_id][0];
          console.log(ref);
          this.draggedId = dev_id;

          // Wait (literally) 2ticks for Vue to internally reorganise and redraw it's DOM
          this.$nextTick(() => {
            // Set the 'placeholder' class back on the original ref to keep it invisible while
            // the restore animation plays.
            ref.$el.classList.add('drag-placeholder');
          });

          // Wait for 300ms for the transform to complete
          setTimeout(() => {
            this.draggedId = null;
            clone.remove()

            let ref = this.$refs[dev_id][0];
            ref.$el.classList.remove('drag-placeholder')
          }, 300);
        }


        // Wait for the DOM to settle before we update PipeWeaver
        this.$nextTick(() => {
          const id = event.source.dataset.id;

          // Grab the list children, then map the order
          const children = Array.from(this.$refs.filters.children);
          const newOrder = children.map(el => el.dataset.id);

          // Locate the 'new' index of this item
          const newIndex = newOrder.indexOf(id);

          // MoveFilter(Ulid, usize),
          let command = {
            "MoveFilter": [id, newIndex]
          }
          websocket.send_command(command);
        })

      })
      this.draggable = draggable;
    },


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

    addFilter(url) {
      let command = {
        "AddFilterToNode": [this.id, {
          "filter": {
            LV2: {
              "plugin_uri": url,
              "values": {}
            }
          }
        }]
      };
      websocket.send_command(command);
    },

    removeFilter(filter) {
      const {id} = this.getFilterInfo(filter);

      if (this.activeFilter?.id === id) {
        this.activeFilter = undefined;
      }

      websocket.send_command({
        RemoveFilter: id
      });
    },

    setActiveFilter(filter) {
      this.activeFilter = this.getFilterInfo(filter);
    },

    getFilters() {
      let device = get_device_by_id(this.id);
      return device.filters;
    },

    getFilterInfo(filter) {
      if (filter.filter['LV2']) {
        return {
          id: filter.id,
          type: 'LV2',
          identifier: filter.filter['LV2'].plugin_uri,
        };
      }
    },

    getFilterName(filter) {
      let id = this.getFilterInfo(filter).id;
      if (store.getAudio().filter_config[id] === undefined) {
        return "Unknown Filter";
      }

      if (this.pluginComponents[store.getAudio().filter_config[id].identifier]) {
        return this.pluginComponents[store.getAudio().filter_config[id].identifier].display;
      }

      return store.getAudio().filter_config[id].name;
    },

    getFilterPageComponent(identifier, filterType) {
      // First try to find a specific component for this plugin URI
      if (this.pluginComponents[identifier]) {
        return this.pluginComponents[identifier].component;
      }

      // Fall back to generic component for this filter type
      return this.fallbackComponents[filterType] || null;
    }
  },

  mounted() {
    this.setup_draggable();
  },

  beforeUnmount() {
    if (this.draggable) {
      this.draggable.destroy();
      this.draggable = null;
    }
  },

  computed: {
    filters() {
      const device = get_device_by_id(this.id);
      return device?.filters || [];
    },

    filterIds() {
      return this.filters.map(f => this.getFilterInfo(f).id);
    },

    currentFilterPageComponent() {
      if (!this.activeFilter) return null;
      return this.getFilterPageComponent(this.activeFilter.identifier, this.activeFilter.type);
    },

    filterState() {
      if (!this.activeFilter) {
        return {state: null, message: null};
      }
      return this.getFilterState(this.activeFilter.id);
    },

    // Label for the action dock's left side, e.g. "Using Calf" / "Using LSP Plugins" -
    // mirrors EasyEffects' own footer text naming which plugin package provides the effect.
    pluginPackageLabel() {
      const uri = this.activeFilter?.identifier || '';
      if (uri.startsWith('http://calf.sourceforge.net')) return 'Calf';
      if (uri.startsWith('http://lsp-plug.in')) return 'LSP Plugins';
      if (uri.startsWith('urn:zamaudio')) return 'ZamAudio';
      return 'LV2';
    }
  },

  watch: {
    filterIds(newIds, oldIds) {
      if (!oldIds) return;

      const addedId = newIds.find(id => !oldIds.includes(id));
      if (!addedId) return;

      const filter = this.filters.find(
        f => this.getFilterInfo(f).id === addedId
      );

      if (filter) {
        this.setActiveFilter(filter);
      }
    }
  },

  provide() {
    return {
      actionBarTarget: `#action-dock-${this.id}`
    };
  },
}
</script>

<template>
  <ModalOverlay body-padding="0" :show_footer="false" ref="filterModal" id="filterViewModal"
                title="Filters" fullWindow window-padding="32px">
    <template v-slot:title>{{ getName() }} - We have EasyEffects at Home</template>
    <template v-slot:default>
      <AddFilterModal ref="addFilterModal" :filters="pluginComponents" @select="addFilter"/>

      <div class="filter-wrapper">
        <div class="filter-list">
          <div class="add-filter" @click="$refs.addFilterModal.open()">Add Filter</div>

          <div ref="filters">
            <FilterListItem
              v-for="filter in getFilters()"
              :key="getFilterInfo(filter).id"
              :ref="getFilterInfo(filter).id"
              :data-id="getFilterInfo(filter).id"
              :filter="filter"
              :filter-info="getFilterInfo(filter)"
              :filter-name="getFilterName(filter)"
              @select="setActiveFilter"
              @remove="removeFilter"
            />
          </div>
        </div>

        <div class="filter-content-pane">
          <div class="filter-scroll-area">
            <div v-if="activeFilter === undefined" class="filter-page empty-state">
              <h3>No Filter Selected</h3>
              <p>Select a filter from the list or add a new one.</p>
              <p class="suggestion">Try:
                <code>http://lsp-plug.in/plugins/lv2/comp_delay_x2_stereo</code></p>
            </div>

            <div v-else class="filter-running-wrap">
              <div v-if="filterState.state === 'Running'" class="filter-running-wrap">
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

          <!-- Persistent status/action dock - mirrors EasyEffects' own footer toolbar: plugin
               source on the left, that filter's action-bar controls teleported in on the
               right. Lives outside .filter-scroll-area so it never scrolls away, and always
               renders (even with an empty right side) rather than only appearing when a
               filter happens to declare an action bar. -->
          <div class="filter-action-dock">
            <div class="dock-source">
              <template v-if="activeFilter">Using <strong>{{ pluginPackageLabel }}</strong>
              </template>
            </div>
            <div class="dock-actions" :id="`action-dock-${id}`"></div>
          </div>
        </div>
      </div>
    </template>
  </ModalOverlay>
</template>

<style scoped>
.filter-wrapper {
  height: 100%;
  min-height: 0;
  display: flex;
  overflow: hidden;
}

.filter-list {
  flex: 0 0 280px;
  display: flex;
  flex-direction: column;
  background: #1e2221;
  border-right: 1px solid #3b403f;
  overflow: hidden;
}

.filter-list > div:last-child {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.filter-list > div:last-child::-webkit-scrollbar {
  width: 8px;
}

.filter-list > div:last-child::-webkit-scrollbar-thumb {
  background: #555;
  border-radius: 8px;
}

.add-filter {
  flex-shrink: 0;
  margin: 0;
  padding: 10px;
  background: #252a29;
  border-bottom: 1px solid #3b403f;
  cursor: pointer;
  text-align: center;
}

.add-filter:hover {
  background: #353a39;
}

/* Right side filter editor */
.filter-content-pane {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.filter-scroll-area {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.filter-scroll-area::-webkit-scrollbar {
  width: 10px;
}

.filter-scroll-area::-webkit-scrollbar-thumb {
  background: #555;
  border-radius: 10px;
}

/* Lets a filter component (e.g. the Equaliser) opt into filling all available height
   instead of just being however tall its content naturally is - height:100% needs an
   unbroken chain of definite heights down from .filter-scroll-area to resolve against. */
.filter-running-wrap {
  height: 100%;
}

.filter-action-dock {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 14px;
  background: #1e2221;
  border-top: 1px solid #3b403f;
}

.dock-source {
  flex-shrink: 0;
  color: #999;
  font-size: 0.9em;
}

.dock-source strong {
  color: #ddd;
}

.dock-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}

/* Empty/error screens */
.empty-state,
.error-state {
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  padding: 40px;
  text-align: center;
  color: #888;
}

.empty-state h3 {
  color: #ddd;
  margin-bottom: 8px;
}

/* Suggestion box */
.suggestion {
  margin-top: 20px;
  padding: 12px;
  background-color: #2a2f2e;
  border: 1px solid #3b403f;
  border-radius: 6px;
}

.suggestion code {
  color: #6bb6ff;
  font-size: 0.9em;
}

/* Drag handler, DO NOT CHANGE ANYTHING BELOW THIS LINE! */
.drag-placeholder {
  position: relative;
}

.drag-placeholder > * {
  visibility: hidden;
}

.drag-placeholder::before {
  content: '';
  position: absolute;
  inset: 0;
  border: 2px dashed #666666;
  pointer-events: none;
}

/* Some minor styling to the mirror */
.custom-mirror {
  position: fixed !important;
  pointer-events: none;
  opacity: 0.6;
}

.custom-mirror-small {
  transition: all 0.3s;

  transform-origin: center center !important;
  will-change: transform, top, left;
}

.restore-mirror {
  position: fixed !important;
  pointer-events: none;
  opacity: 0.6;
  z-index: 9999;
  transition: transform 0.3s;
  transform-origin: center center !important;
  will-change: transform;
}

/* Some minor styling to the mirror */
.custom-mirror {
  position: fixed !important;
  pointer-events: none;
  opacity: 0.6;
}
</style>
