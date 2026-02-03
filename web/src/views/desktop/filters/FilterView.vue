<script>
import ModalOverlay from "@/views/desktop/components/ModalOverlay.vue";
import {get_device_by_id} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";
import {store} from "@/app/store.js";

export default {
  name: "FilterView",
  components: {ModalOverlay},
  props: {
    id: {type: String, required: true}
  },

  data() {
    return {
      activeFilter: undefined,
    }
  },

  methods: {
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

      // AddFilterToNode(Ulid, Filter),
      // pub enum Filter {
      //     LV2(LV2Filter),
      // }
      // pub struct LV2Filter {
      //   #[serde(default = "generate_uid")]
      //   pub id: Ulid,
      //
      //   pub plugin_uri: String,
      //   pub values: HashMap<String, FilterValue>,
      // }
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

    setActiveFilter(filter) {
      this.activeFilter = this.getFilterId(filter);
    },

    getFilters() {
      let device = get_device_by_id(this.id);
      return device.filters;
    },

    getFilterId(filter) {
      if (filter['LV2']) {
        return filter['LV2'].id;
      }
    },

    getFilterName(filter) {
      let id = this.getFilterId(filter);
      if (store.getAudio().filter_config[id] === undefined) {
        return "Unknown Filter";
      }

      return store.getAudio().filter_config[id].name;
    },

    getFilterProperties(id) {
      if (store.getAudio().filter_config[id] === undefined) {
        return [];
      }
      
      return store.getAudio().filter_config[id].parameters.filter(prop => prop.is_input !== false);
    },

    getFilterPropertyType(prop) {
      // We should support all of these, but for now, we'll just support the LV2 basics
      /*
        pub enum FilterValue {
            Int32(i32),
            Float32(f32),
            UInt8(u8),
            UInt32(u32),
            String(String),
            Bool(bool),
            Enum(String, u32),
        }
      */

      // First check for enum, because this returns as an Int32
      if (prop['enum_def'] !== null) {
        return 'enum';
      }
      console.log(prop);

      if (prop['value']['Bool'] !== undefined) {
        return 'bool';
      }
      if (prop['value']['Int32'] !== undefined) {
        return 'int';
      }
      if (prop['value']['Float32'] !== undefined) {
        return 'float';
      }

      // if prop.enu
      //
      // if prop.value[]
    }
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
          <div v-for="filter in getFilters()" class="filter-item" @click="setActiveFilter(filter)">
            {{ getFilterName(filter) }}
          </div>
        </div>
        <div v-if="activeFilter === undefined" class="filter-page">Need Dis:
          http://lsp-plug.in/plugins/lv2/comp_delay_x2_stereo
        </div>
        <div class="filter-page" v-else>
          <div class="prop-value">
            <div v-for="prop of getFilterProperties(activeFilter)">
              <span class="prop-label">{{ prop.name }}</span>
              <span v-if="getFilterPropertyType(prop) === 'bool'" class="prop-value">
                <input type="checkbox" :checked="prop.value.Bool"/>
              </span>
              <span v-else-if="getFilterPropertyType(prop) === 'int'" class="prop-value">
                <input type="number" :value="prop.value.Int32" :min="prop.min" :max="prop.max"/>
              </span>
              <span v-else-if="getFilterPropertyType(prop) === 'float'" class="prop-value">
                <input type="number" step="0.01" :value="prop.value.Float32" :min="prop.min"
                       :max="prop.max"/>
              </span>
              <span v-else-if="getFilterPropertyType(prop) === 'enum'" class="prop-value">
                <select>
                  <option v-for="(enum_name, enum_value) in prop.enum_def" :value="enum_value">
                    {{ enum_name }}
                  </option>
                </select>
              </span>
              <span v-else>
                Err: {{ getFilterProperties(activeFilter) }}
              </span>
            </div>
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

.filter-item {
  cursor: pointer;
  padding: 10px;
}

.filter-item:hover {
  background-color: #353a39;
}

.prop-label {
  display: inline-block;
  width: 200px;
}

.prop-value input[type="number"] {
  width: 100px;
}

.prop-value select {
  width: 100px;
}


</style>
