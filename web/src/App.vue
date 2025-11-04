<script setup>
import {computed} from 'vue';
import {runWebsocket} from '@/app/sockets.js';
import {store} from '@/app/store.js';

import LoadingScreen from '@/views/LoadingScreen.vue';
import HomePage from '@/views/HomePage.vue';
import {useDeviceType, useOrientation} from "@/app/util.js";

// Run once on component setup
runWebsocket();

const isConnected = computed(() => store.isConnected());
const {isDesktop} = useDeviceType();
const {isPortrait} = useOrientation();
</script>

<template>
  <LoadingScreen v-if="!isConnected"/>
  <HomePage v-else/>
  <!--
    <HomePage v-else-if="isDesktop"/>
    <PortraitPage v-else-if="isPortrait"/>
    <LandscapePage v-else/>
  -->
</template>

<style scoped>

</style>
