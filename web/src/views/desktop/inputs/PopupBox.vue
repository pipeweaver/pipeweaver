<script>
export default {
  name: "PopupBox",

  props: {},

  data() {
    return {
      is_active: false,
      suppress_click: true,
      identifier: null,
      last_position_params: null,
    };
  },

  mounted() {
    window.addEventListener('resize', this.repositionDialog);
  },

  beforeUnmount() {
    window.removeEventListener('resize', this.repositionDialog);
  },

  methods: {
    showDialog(source_event, identifier, scrollTop, bottom_aligned) {
      if (scrollTop === undefined) {
        scrollTop = 0;
      }

      let positionElement = null;
      if (source_event.target === undefined) {
        positionElement = source_event;
      } else {
        positionElement = source_event.target;
      }

      // If it's an SVG or Path, we need to locate the containing div...
      let found = false;
      while (!found) {
        if (positionElement.nodeName === "svg" || positionElement.nodeName === "path") {
          positionElement = positionElement.parentNode;
          continue;
        }
        found = true;
      }

      this.identifier = identifier;
      this.last_position_params = {
        element: positionElement,
        scrollTop: scrollTop,
        bottom_aligned: bottom_aligned,
      };

      this.is_active = true;
      this.$nextTick(() => {
        this.repositionDialog();
      });
    },

    repositionDialog() {
      if (!this.is_active || !this.last_position_params) {
        return;
      }

      const SCREEN_PADDING = 5;
      const {element, scrollTop, bottom_aligned} = this.last_position_params;
      const container = this.$refs.container;
      let menuWidth = container.offsetWidth;
      let menuHeight = container.offsetHeight;

      let left = element.offsetLeft;
      let top = element.offsetTop - scrollTop;

      if (bottom_aligned) {
        top += element.clientHeight;
      } else {
        left += element.clientWidth;
        top += (element.clientHeight / 2);
      }

      let windowScrollTop = document.documentElement.scrollTop || document.body.scrollTop;

      // Check horizontal boundaries (use padding only for screen edges)
      const maxRight = window.innerWidth - SCREEN_PADDING;
      if (left + menuWidth >= maxRight) {
        // Flip to the left of the element (no extra screen padding between element and popup)
        left = element.offsetLeft - menuWidth;
      }

      // Check vertical boundaries
      const maxBottom = window.innerHeight + windowScrollTop - SCREEN_PADDING;
      if (top + menuHeight >= maxBottom) {
        if (bottom_aligned) {
          // Place above the element (no extra gap between element and popup)
          top = element.offsetTop - scrollTop - menuHeight;
        } else {
          // Clamp to bottom of screen with screen padding
          top = maxBottom - menuHeight;
        }
      }

      // Ensure popup doesn't go off left edge (screen padding)
      if (left < SCREEN_PADDING) {
        left = SCREEN_PADDING;
      }

      // Ensure popup doesn't go off top edge (screen padding)
      if (top < windowScrollTop + SCREEN_PADDING) {
        top = windowScrollTop + SCREEN_PADDING;
      }

      container.style.left = left + "px";
      container.style.top = top + "px";
    },

    close() {
      this.hideDialog();
    },

    hideDialog() {
      if (this.is_active) {
        this.is_active = false;
        this.suppress_click = true;
        this.last_position_params = null;
        this.$emit('closed', this.identifier);
      }
    },

    onClickOutside() {
      if (this.is_active) {
        if (this.suppress_click) {
          this.suppress_click = false;
          return;
        }
        this.hideDialog();
      }
    }
  }
}
</script>

<template>
  <div v-show="is_active" ref="container" v-click-outside="onClickOutside" class="container">
    <slot></slot>
  </div>
</template>

<style scoped>
.container {
  background-color: #252927;
  color: #fff;
  border: 1px solid #6e7676;
  list-style: none;
  position: absolute;
  left: 0;
  margin: 0;
  padding: 0;
  top: 0;
  z-index: 100;
  box-shadow: 2px 4px 6px rgba(0, 0, 0, 0.3);
}

</style>
