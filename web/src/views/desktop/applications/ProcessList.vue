<script>
import {store} from "@/app/store.js";
import Application from "@/views/desktop/applications/Application.vue";

export default {
  name: "ProcessList",
  components: {Application},

  props: {
    isSource: {type: Boolean, required: true},
  },

  methods: {
    get_source_key() {
      return this.isSource ? "Source" : "Target";
    },

    get_process_list() {
      return Object.keys(store.getApplications()[this.get_source_key()]).sort()
    }
  }
}
</script>

<template>
  <div v-for="process in get_process_list()" class="process">
    <div class="title">{{ process }}</div>
    <Application :is-source="this.isSource" :process-name="process"/>
  </div>
</template>

<style scoped>
.process {
  border: 1px solid #3b413f;
}

.process .title {
  padding: 5px 5px 5px 8px;
  background-color: #3b413f;
  border-bottom: 1px solid #3b413f;
}
</style>
