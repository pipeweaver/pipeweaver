<script>
import {store} from "@/app/store.js";
import Application from "@/views/desktop/applications/Application.vue";

export default {
  name: "ProcessList",
  components: {Application},

  props: {
    isSource: {type: Boolean, required: true},
  },

  data() {
    return {
      cached_apps: {},
      watch: null
    }
  },

  methods: {
    get_source_key() {
      return this.isSource ? "Source" : "Target";
    },

    get_process_list() {
      const store_list = store.getApplications()[this.get_source_key()] || {};
      const store_keys = Object.keys(store_list);
      const cached_keys = Object.keys(this.cached_apps);

      // Merge the 'Store' and the 'Cached' keys (Use a Set for deduplication)
      const union = new Set([...store_keys, ...cached_keys]);

      // Create an array from them, and sort.
      return Array.from(union).sort();
    },

    onRequestRemove(processName, appName) {
      const cached_app = this.cached_apps[processName];
      if (!cached_app) return;

      const idx = cached_app.indexOf(appName);
      if (idx !== -1) {
        cached_app.splice(idx, 1);

        // Clean up empty process if it doesn't exist in store
        if (cached_app.length === 0) {
          const store_list = store.getApplications()[this.get_source_key()] || {};
          if (!store_list[processName]) {
            delete this.cached_apps[processName];
          }
        }
      }
    },

    syncApps() {
      const app_list = store.getApplications()[this.get_source_key()] || {};

      for (const process of this.get_process_list()) {
        const store_apps = Object.keys(app_list[process] || {});

        if (store_apps.length > 0) {
          this.cached_apps[process] = this.cached_apps[process]
            ? [...new Set([...this.cached_apps[process], ...store_apps])]
            : store_apps;
        }
      }

      // Clean up empty processes
      const valid_processes = new Set(Object.keys(app_list));
      for (const process of Object.keys(this.cached_apps)) {
        if (!valid_processes.has(process) && this.cached_apps[process]?.length === 0) {
          delete this.cached_apps[process];
        }
      }
    }
  },

  mounted() {
    const key = this.get_source_key();

    // Create a watcher for the applications in the store that match our key, and callback on change
    this.watch = this.$watch(
      () => JSON.stringify(store.getApplications()[key] || {}),
      () => this.syncApps(),
      {immediate: true}
    );
  },

  beforeUnmount() {
    if (this.watch) {
      this.watch();
      this.watch = null;
    }
  }
}
</script>

<template>
  <div v-for="process in get_process_list()" :key="process" class="process">
    <div class="title">{{ process }}</div>
    <Application
      :apps="cached_apps[process] || []"
      :is-source="this.isSource"
      :process-name="process"
      @request-remove="app => onRequestRemove(process, app)"
    />
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
