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
</style>
