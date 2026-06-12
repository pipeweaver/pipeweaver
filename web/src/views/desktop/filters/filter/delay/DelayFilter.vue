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
