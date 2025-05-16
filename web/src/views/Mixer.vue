<script>
import ChannelColumn from "@/components/channels/ChannelColumn.vue";
import {DeviceType, get_device_order, get_device_type} from "@/app/util.js";
import {Sortable} from "@shopify/draggable";
import {websocket} from "@/app/sockets.js";
import PopupBox from "@/components/inputs/PopupBox.vue";

const INTERNAL_SCALE = 0.8;

export default {
  name: "Mixer",
  components: {PopupBox, ChannelColumn},

  props: {
    is_source: Boolean,
  },
  data() {
    return {
      deviceOrder: get_device_order(this.is_source),
      draggedId: null,
      draggable: null,
    };
  },
  methods: {
    get_device_type,

    show_popup(e) {
      this.$refs.popup.showDialog(e)
    },

    add_device(is_physical) {
      this.$refs.popup.hideDialog();
      let name = prompt("Device Name:");

      if (name === undefined || name === "" || name === null) {
        return;
      }

      // We need to break down the type
      let final_type = undefined;
      if (is_physical) {
        if (this.is_source) {
          final_type = DeviceType.PhysicalSource;
        } else {
          final_type = DeviceType.PhysicalTarget;
        }
      } else {
        if (this.is_source) {
          final_type = DeviceType.VirtualSource;
        } else {
          final_type = DeviceType.VirtualTarget;
        }
      }

      // CreateNode(NodeType, String),
      let command = {
        "CreateNode": [final_type, name]
      }
      websocket.send_command(command)
    },

    setup_draggable() {
      if (this.draggable) {
        this.draggable.destroy();
        this.draggable = null;
      }

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
        event.source.dataset.originalLeft = event.source.getBoundingClientRect().left.toString();
        this.draggedId = event.source.dataset.id;
      })

      draggable.on('mirror:created', (event) => {
        event.source.classList.add('drag-placeholder');
        event.mirror.classList.add('custom-mirror');
        document.body.classList.add('dragging');
      });

      draggable.on('drag:move', (event) => {
        const mirror = document.querySelector('.draggable-mirror');
        const placeholder = document.querySelector('.drag-placeholder');

        if (!mirror || !placeholder) return;
        let positionX = placeholder.getBoundingClientRect().left;
        let positionY = placeholder.getBoundingClientRect().top;

        mirror.style.transform = `translate3d(${positionX}px, ${positionY}px, 0px) scale(${INTERNAL_SCALE})`;
        if (!mirror.classList.contains("custom-mirror-small")) {
          mirror.classList.add("custom-mirror-small");
        }
      });

      draggable.on('drag:stop', (event) => {
        const mirror = document.querySelector('.draggable-mirror');
        const dev_id = event.source.dataset.id;
        if (mirror) {

          // Clone the mirror to keep visible after drag ends
          const clone = mirror.cloneNode(true);
          clone.style.cssText = mirror.style.cssText;
          const appRoot = document.getElementById('app');
          appRoot.appendChild(clone);

          const placeholder = document.querySelector('.drag-placeholder');
          if (!placeholder) return;

          let localX = event.source.getBoundingClientRect().left;
          let localY = event.source.getBoundingClientRect().top;

          // Animate fadeout and remove clone after delay
          clone.style.transform = `translate3d(${localX}px, ${localY}px, 0px) scale(1)`;

          // Remove the cursor drag icon
          document.body.classList.remove('dragging');

          // Forcibly re-add the placeholder class to the target location
          let ref = this.$refs[dev_id][0];
          this.draggedId = dev_id;

          // Wait (literally) 2ticks for Vue to internally reorganise and redraw it's DOM
          this.$nextTick(() => {
            // Set the 'placeholder' class back on the original ref to keep it invisible while
            // the restore animation plays.
            ref.classList.add('drag-placeholder');
          });

          // Wait for 300ms for the transform to complete
          setTimeout(() => {
            this.draggedId = null;
            clone.remove()

            let ref = this.$refs[dev_id][0];
            ref.classList.remove('drag-placeholder')
          }, 300);
        }


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
  },

  computed: {
    deviceOrderKey() {
      return this.deviceOrder.join("-");
    }
  },
  watch: {
    deviceOrderKey() {
      this.$nextTick(() => {
        if (this.draggedId) {
          let ref = this.$refs[this.draggedId][0];
          ref.classList.add('drag-placeholder');
        }
        this.setup_draggable();
      })
    }
  },
  mounted() {
    this.setup_draggable();
  },
  beforeUnmount() {
    if (this.draggable) {
      this.draggable.destroy();
      this.draggable = null;
    }
  }

}
</script>

<template>
  <PopupBox ref="popup" @closed="">
    <div class="entry" @click="add_device(false)">
      <span>Add Virtual Channel</span>
    </div>
    <div class="separator"/>
    <div class="entry" @click="add_device(true)">
      <span>Add Physical Channel</span>
    </div>
  </PopupBox>

  <div class="mix-list">
    <div class="title">
      <div class="start"></div>
      <div class="text">{{ is_source ? "Sources" : "Targets" }}</div>
      <div class="end">
        <button @click="show_popup">+</button>
      </div>
    </div>

    <div :key="deviceOrderKey" ref="deviceList" class="mixer">
      <div v-for="id of deviceOrder" :key="id" :ref="id"
           :data-id="id" class="channel-column">
        <ChannelColumn :id="id" :type="get_device_type(id)"/>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mix-list {
  display: flex;
  flex-direction: column;
  border: 1px solid #666;
  border-radius: 6px 6px 0 0;
  background-color: #2d3230;
}

.mix-list .title {
  font-weight: bold;
  text-align: center;
  padding-top: 10px;

  display: flex;
  flex-direction: row;
}

.mix-list .title .start {
  width: 40px;
}

.mix-list .title .text {
  flex: 1;
}

.mix-list .title .end {
  width: 40px;
}

.mix-list .title .end button {
  all: unset;
  height: 20px;
  width: 20px;
  color: #fff;
  border: 1px solid #666666;
  background-color: #353937;
  border-radius: 5px;
}

.mix-list .title .end button:hover {
  cursor: pointer;
}

.mixer {
  flex: 1;
  background-color: #2d3230;
  padding: 10px;
  display: flex;
  gap: 15px;
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
}

.custom-mirror-small {
  transition: all 0.3s;

  transform-origin: center center !important;
  will-change: transform, top, left;
}

.separator {
  height: 5px;
  background-color: #3b413f;
}

.title {
  white-space: nowrap;
}

.entry {
  white-space: nowrap;
  padding: 6px 20px 6px 20px;
}

.entry:hover {
  background-color: #49514e;
  cursor: pointer;
}

.entry:not(:last-child) {
  border-bottom: 1px solid #3b413f;
}

</style>
