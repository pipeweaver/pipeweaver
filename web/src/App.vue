<script setup>
import {runWebsocket} from "@/app/sockets.js";
import LoadingScreen from "@/views/LoadingScreen.vue";
import {store} from "@/app/store.js";
import HomePage from "@/views/HomePage.vue";

// Start the Main Pipecast Websocket
runWebsocket()

</script>

<template>
  <LoadingScreen v-if="!store.isConnected()"/>
  <HomePage v-if="store.isConnected()"/>
</template>

<style scoped>

</style>
