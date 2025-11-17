<script>
import ProcessList from "@/views/desktop/applications/ProcessList.vue";

export default {
  name: "applications",
  components: {ProcessList},

  methods: {}
}
</script>

<template>
  <div class="applications">
    <div>
      <b>Output</b>
      <ProcessList :is-source="true"/>
    </div>
    <div>
      <b>Input</b>
      <ProcessList :is-source="false"/>
    </div>
  </div>
</template>

<style scoped>
.applications {
  padding: 10px;
  min-height: 200px;
  display: flex;
  flex-direction: row;
  gap: 25px;
}

.applications div {
  min-width: 300px;
  max-width: 300px;
}
</style>
