<script>
import ChannelColumn from "@/components/channels/ChannelColumn.vue";
import {get_device_order, get_device_type} from "@/app/util.js";
import {Sortable} from "@shopify/draggable";
import {websocket} from "@/app/sockets.js";

export default {
  name: "Mixer",
  components: {ChannelColumn},

  props: {
    is_source: Boolean,
  },
  data() {
    return {
      deviceOrder: get_device_order(this.is_source),
      dragOffsetX: 0,
    };
  },
  methods: {
    get_device_type,

    handleSort() {
      console.log("Device order updated:", this.deviceOrder);
    },
  },

  mounted() {
    const container = this.$refs.deviceList;
    const draggable = new Sortable(container, {
      draggable: '.channel-column',
      handle: '.drag-handle',
      delay: 0,

      mirror: {
        xAxis: false,
        yAxis: false,
        constrainDimensions: true,
      },
    });

    draggable.on('drag:start', (event) => {

    })

    draggable.on('mirror:created', (event) => {
      event.source.classList.add('drag-placeholder');
      event.mirror.classList.add('custom-mirror');
      document.body.classList.add('dragging');
    });

    draggable.on('drag:move', (event) => {
      const mirror = document.querySelector('.draggable-mirror');
      if (!mirror) return;

      const placeholder = document.querySelector('.drag-placeholder');
      if (!placeholder) return;

      const rect = placeholder.getBoundingClientRect();

      // We want to key the Y axis central to the parent
      const positionY = rect.top;
      const positionX = rect.left;

      mirror.style.left = `${positionX}px`;
      mirror.style.top = `${positionY}px`;
    });

    draggable.on('drag:stop', (event) => {
      event.source.classList.remove('drag-placeholder');
      document.body.classList.remove('dragging');

      // Wait for the DOM to settle before we update PipeWeaver
      this.$nextTick(() => {
        const id = event.source.dataset.id;

        // Grab the list children, then map the order
        const children = Array.from(this.$refs.deviceList.children);
        const newOrder = children.map(el => el.dataset.id);

        // Locate the 'new' index of this item
        const newIndex = newOrder.indexOf(id);

        // Send the message to the websocket that we've reordered
        let command = {
          "SetOrder": [id, newIndex]
        }
        websocket.send_command(command);
      })

    })


    this.draggable = draggable;
  }
}
</script>

<template>
  <div ref="deviceList" class="mixer">
    <div v-for="id of deviceOrder" :key="id" :data-id="id" class="channel-column">
      <ChannelColumn :id="id" :type="get_device_type(id)"/>
    </div>
  </div>
</template>

<style scoped>
.mixer {
  background-color: #2d3230;
  padding: 15px;
  display: flex;
  gap: 15px;
}

.mixer button {
  color: #fff;
  border: 1px solid #666666;
  background-color: #353937;
  border-radius: 5px;
}

.mixer button:hover {
  background-color: #49514e;
  cursor: pointer;
}

/* We need to hide the contents of the original to so we can instead use a placeholder */
.drag-placeholder {
  position: relative;
}

.drag-placeholder > * {
  visibility: hidden;
}

.drag-placeholder::before {
  content: '';
  position: absolute;
  inset: 0;
  border: 2px dashed #666666;
  pointer-events: none;
}

/* Some minor styling to the mirror */
.custom-mirror {
  position: fixed !important;
  pointer-events: none;
  opacity: 0.6;

  transform-origin: center center !important;
  transform: scale(0.8) !important;
  will-change: transform, top, left;
}


</style>
