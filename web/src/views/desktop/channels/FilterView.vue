<script>
import ModalOverlay from "@/views/desktop/components/ModalOverlay.vue";
import {get_device_by_id} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";

export default {
  name: "FilterView",
  components: {ModalOverlay},
  props: {
    id: {type: String, required: true}
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
          <div class="add_filter" @click="addFilter">Add Filter</div>
        </div>
        <div class="filter-page">Need Dis: http://lsp-plug.in/plugins/lv2/comp_delay_x2_stereo</div>
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

.add_filter {
  margin: 0 5px 5px 5px;
  padding: 10px;
  border-bottom: 1px solid #fff;
  cursor: pointer;
  text-align: center;
}

.add_filter:hover {
  background-color: #353a39;
}

</style>
