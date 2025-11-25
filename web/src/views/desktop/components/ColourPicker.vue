<script>
import PopupBox from "@/views/desktop/inputs/PopupBox.vue";

export default {
  name: "ColourPicker",
  components: {PopupBox},

  data() {
    return {
      hexString: "#000000",
      canvasContext: undefined,
      hoverContainer: undefined,

      active: false,
      outside: false,
    }
  },

  props: {
    id: {type: String, required: true},
    title: String,
    colourValue: String,
  },

  methods: {
    show(e) {
      //this.active = true;
      this.$refs.popup.showDialog(e, this.id, undefined, true)
    },

    mouseDown() {
      this.$refs.target.classList.add("active");
      this.active = true;
    },

    mouseOut() {
      this.$refs.target.classList.remove("active");
      this.active = false;

      // Check if the cursor is outside the image...
      if (this.outside) {
        this.hoverContainer.style.left = "-30px";
        this.hoverContainer.style.top = "-30px";
      }
    },

    mouseUp(event) {
      if (!this.active) {
        return;
      }

      this.$refs.target.classList.remove("active");
      this.active = false;

      if (this.outside) {
        this.hoverContainer.style.left = "-30px";
        this.hoverContainer.style.top = "-30px";

        // We need to set the colour released..
        let position = this.getOutsidePosition(event);
        const colour = this.colour(position[0], position[1]);
        this.updateText(colour);
      }
    },

    mouseMoveOutside(event) {
      const hoverContainerCenterOffset = 12

      if (this.active) {
        // Firstly, calculate the central position of the image (top spacer + half height)
        let position = this.getOutsidePosition(event);

        // Shove the dot there.
        this.hoverContainer.style.left = (position[2] - hoverContainerCenterOffset) + "px";
        this.hoverContainer.style.top = (position[3] - hoverContainerCenterOffset) + "px";

        // Update the colour for these coordinates..
        let colour = this.colour(position[0], position[1]);
        this.hoverContainer.style.backgroundColor = colour;
        if (this.active) {
          this.updateText(colour);
        }
      }
    },

    getOutsidePosition(event) {
      let radius = 60;

      let middleY = this.$refs.target.clientHeight / 2;
      let middleX = this.$refs.target.clientWidth / 2;

      let offset = this.$refs.target.offsetParent;

      // These are the relative X,Y relative to the middle of the image
      let relativeX = (event.pageX - offset.offsetLeft - middleX);
      let relativeY = (event.pageY - offset.offsetTop - middleY);

      // Get the distance between the middle and our cursor
      let distance = Math.sqrt(Math.pow(relativeX, 2) + Math.pow(relativeY, 2));
      let normalX = relativeX / distance;
      let normalY = relativeY / distance;

      let resultX = (normalX * radius);
      let resultY = (normalY * radius);

      let positionX = offset.offsetLeft + middleX + resultX;
      let positionY = offset.offsetTop + middleY + resultY;

      return [resultX + radius, resultY + radius, positionX, positionY];
    },

    mouseMove(event) {
      const hoverContainerCenterOffset = 12

      this.hoverContainer.style.left = (event.pageX - hoverContainerCenterOffset) + "px";
      this.hoverContainer.style.top = (event.pageY - hoverContainerCenterOffset) + "px";

      const position = this.position(event)
      const colour = this.colour(position[0], position[1]);
      this.hoverContainer.style.backgroundColor = colour;

      if (this.active) {
        this.updateText(colour);
      }
    },

    mouseLeave() {
      if (!this.active) {
        this.hoverContainer.style.left = "-30px";
        this.hoverContainer.style.top = "-30px";
      }
      this.outside = true;
    },

    mouseClick(event) {
      const position = this.position(event)
      const colour = this.colour(position[0], position[1])
      this.updateText(colour);
    },

    position(event) {
      let rect = event.target.getBoundingClientRect();
      let x = Math.floor(event.clientX - rect.left);
      let y = Math.floor(event.clientY - rect.top);

      return [x, y];
    },

    colour(x, y) {
      let colour = this.canvasContext.getImageData(x, y, 1, 1).data;
      return "#" + ("000000" + this.hexColour(colour[0], colour[1], colour[2])).slice(-6).toUpperCase();
    },

    hexColour(r, g, b) {
      if (r > 255 || g > 255 || b > 255)
        throw "Invalid colour component";
      return ((r << 16) | (g << 8) | b).toString(16);
    },

    updateText(value) {
      this.hexString = value;
      this.$emit('colour-changed', this.hexString);
    },

    updateColour(event) {
      let value = event.target.value;

      const regex = /^#([a-fA-Z0-9]{6})\b$/
      if (value.match(regex)) {
        this.updateText(value);
      }
    },

    clearColour() {
      this.updateText("#000000");
    },

    onClosed(e) {
      this.active = false;
      this.$emit('closed', e);
    },

    previewClicked(e) {
      this.$emit('preview-clicked', e);
    }
  },

  mounted() {
    this.canvasContext = document.getElementById('wheelCanvas').getContext("2d");
    this.hoverContainer = document.getElementById("colourHover");
    this.hexString = this.colourValue;
  },

  watch: {
    colourValue: function () {
      this.hexString = this.colourValue;
    }
  }
}
</script>

<template>
  <PopupBox ref="popup" @close="onClosed">
    <div ref="target" class="colourTarget" @mouseleave="mouseOut" @mousemove="mouseMoveOutside"
         @mouseup="mouseUp">
      <div class="spacer"></div>
      <img ref="circle" :aria-label="`${title}, Colour Picker`" alt="colour wheel" draggable="false"
           role="button"
           src="/wheel.png" tabindex="0"
           @click="mouseClick" @mousedown="mouseDown" @mouseleave="mouseLeave"
           @mousemove.stop="mouseMove"/>
      <div class="spacer"></div>
    </div>
    <div class="controls">
      <div class="colourPreview" @click="previewClicked"></div>
      <input :aria-label="title" :value="hexString" type="text" @keyup="updateColour"/>

      <button :aria-label="`Clear ${title}`" @click="clearColour">
        <font-awesome-icon icon="fa-solid fa-xmark" title="Clear"/>
      </button>
    </div>
  </PopupBox>
</template>

<style scoped>
* {
  margin: 0;
  padding: 0;
}

.spacer {
  height: 14px;
}

.controls {
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  align-items: center;

  height: 35px;
  width: 100%;

  background-color: #3b413f;
  color: #59b1b6;
}

.colourPreview {
  height: 100%;
  width: 33px;

  cursor: pointer;
  background-color: v-bind(hexString);
}

button {
  height: 100%;
  width: 35px;

  color: #ffffff;
  background-color: transparent;
  border: none;
  cursor: pointer;
}

input[type=text] {
  width: 6em;

  color: #59b1b6;
  background-color: #3b413f;
  border: none;
  text-align: center;

  appearance: textfield;
}

img {
  height: 120px;
  width: 120px;
  border-radius: 50%;
}

img:hover {
  cursor: none;
}

.colourTarget {
  width: 100%;
  text-align: center;
}

</style>
