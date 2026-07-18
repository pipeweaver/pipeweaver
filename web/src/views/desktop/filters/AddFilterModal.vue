<script>
import ModalOverlay from "@/views/desktop/components/ModalOverlay.vue";

export default {
  name: "AddFilterModal",

  components: {
    ModalOverlay,
  },

  props: {
    filters: {
      type: Object,
      required: true,
    },
  },

  data() {
    return {
      search: "",
    };
  },

  computed: {
    filteredFilters() {
      const query = this.search.toLowerCase();

      return Object.entries(this.filters)
        .map(([uri, filter]) => ({
          uri,
          name: filter.display,
        }))
        .filter(filter =>
          filter.name.toLowerCase().includes(query)
        )
        .sort((a, b) =>
          a.name.localeCompare(b.name)
        );
    },
  },

  methods: {
    open() {
      this.search = "";
      this.$refs.modal.openModal(undefined, undefined);
    },

    selectFilter(uri) {
      this.$emit("select", uri);
      this.$refs.modal.closeModal();
    },
  },
};
</script>

<template>
  <ModalOverlay
    ref="modal"
    id="addFilterModal"
    title="Add Filter"
    :show_footer="false"
    width="500px"
  >
    <template v-slot:title>Add Filter</template>
    <div class="add-filter-modal">
      <input
        v-model="search"
        class="filter-search"
        placeholder="Search filters..."
        autofocus
      />

      <div class="filter-list">
        <button
          v-for="filter in filteredFilters"
          :key="filter.uri"
          class="filter-entry"
          @click="selectFilter(filter.uri)"
        >
          {{ filter.name }}
        </button>

        <div v-if="filteredFilters.length === 0" class="empty">
          No matching filters
        </div>
      </div>
    </div>
  </ModalOverlay>
</template>

<style scoped>
.add-filter-modal {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.filter-search {
  flex-shrink: 0;
  padding: 10px;
  margin-bottom: 10px;

  background: #1e2221;
  color: white;

  border: 1px solid #555;
  border-radius: 4px;

  font-size: 1rem;
}

.filter-list {
  flex: 1;
  min-height: 0;

  overflow-y: auto;

  display: flex;
  flex-direction: column;
  gap: 4px;
}

.filter-entry {
  padding: 10px;

  text-align: left;

  background-color: #293a4f;
  //background: #252a29;
  color: white;

  border: var(--border);
  border-radius: 4px;

  cursor: pointer;
}

.filter-entry:hover {
  background: #324660;
  border-color: #435e82;
}

.empty {
  padding: 20px;
  text-align: center;
  color: #888;
}
</style>
