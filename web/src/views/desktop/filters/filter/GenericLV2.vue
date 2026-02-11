<script>
import {store} from "@/app/store.js";
import {websocket} from "@/app/sockets.js";

export default {
  name: "GenericLV2FilterPage",
  props: {
    filterId: {type: String, required: true},
    filterType: {type: String, required: true}
  },

  methods: {
    getFilterProperties(id) {
      if (store.getAudio().filter_config[id] === undefined) {
        return [];
      }

      return store.getAudio().filter_config[id].parameters.filter(prop => prop.is_input !== false);
    },

    getFilterPropertyType(prop) {
      if (prop['enum_def'] !== null) {
        return 'enum';
      }

      if (prop['value']['Bool'] !== undefined) {
        return 'bool';
      }
      if (prop['value']['Int32'] !== undefined) {
        return 'int';
      }
      if (prop['value']['Float32'] !== undefined) {
        return 'float';
      }
    },

    setFilterPropertyValue(filter_id, prop_id, e) {
      let input = store.getAudio().filter_config[filter_id];
      if (input === undefined) {
        console.error("Unknown filter ID: " + filter_id);
        return;
      }

      let prop = input.parameters.find(p => p.id === prop_id);
      let prop_type = this.getFilterPropertyType(prop);

      let raw = e.target.value;
      let send_value = undefined;

      if (prop_type === 'bool') {
        let value = e.target.checked;
        send_value = {"Bool": value};
      } else if (prop_type === 'int' || prop_type === 'enum') {
        let value = parseInt(raw, 10);
        send_value = {"Int32": value};
      } else if (prop_type === 'float') {
        let value = parseFloat(raw);
        send_value = {"Float32": value};
      } else {
        console.error("Unsupported filter property type: " + prop_type);
        return;
      }

      let command = {
        "SetFilterValue": [filter_id, prop_id, send_value]
      };
      websocket.send_command(command);
    }
  },

  computed: {
    filterState() {
      return this.getFilterState(this.filterId);
    },

    pluginName() {
      const config = store.getAudio().filter_config[this.filterId];
      return config?.name || 'Unknown Plugin';
    }
  }
}
</script>

<template>
  <div class="generic-filter-page">
    <h3>{{ pluginName }}</h3>
    <div class="plugin-uri">{{ filterType }}</div>

    <div>
      <div v-for="(prop, id) in getFilterProperties(filterId)" :key="id" class="param-row">
        <span class="prop-label">{{ prop.name }}</span>
        <span v-if="getFilterPropertyType(prop) === 'bool'" class="prop-value">
          <input type="checkbox" :checked="prop.value.Bool"
                 @change="e => setFilterPropertyValue(filterId, prop.id, e)"/>
        </span>
        <span v-else-if="getFilterPropertyType(prop) === 'int'" class="prop-value">
          <input type="number" :value="prop.value.Int32" :min="prop.min" :max="prop.max"
                 @change="e => setFilterPropertyValue(filterId, prop.id, e)"/>
        </span>
        <span v-else-if="getFilterPropertyType(prop) === 'float'" class="prop-value">
          <input type="number" step="0.01" :value="prop.value.Float32" :min="prop.min"
                 :max="prop.max"
                 @change="e => setFilterPropertyValue(filterId, prop.id, e)"/>
        </span>
        <span v-else-if="getFilterPropertyType(prop) === 'enum'" class="prop-value">
          <select @change="e => setFilterPropertyValue(filterId, prop.id, e)">
            <option v-for="(enum_name, enum_value) in prop.enum_def" :value="enum_value"
                    :key="enum_value">
              {{ enum_name }}
            </option>
          </select>
        </span>
        <span v-else>
          Unsupported type
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.generic-filter-page {
  width: 800px;
  padding: 20px;
}

.plugin-uri {
  font-size: 0.8em;
  color: #888;
  margin-bottom: 20px;
}

.param-row {
  display: flex;
  gap: 10px;
  align-items: center;
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
