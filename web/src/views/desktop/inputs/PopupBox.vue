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
      if (!container || !element) return;

      // Get stable floats, then convert sizes to integer pixels for all comparisons
      const containerRect = container.getBoundingClientRect();
      const menuWidth = Math.ceil(containerRect.width);
      const menuHeight = Math.ceil(containerRect.height);

      const elementRect = element.getBoundingClientRect();

      // Compute desired float coords
      let leftF, topF;
      if (bottom_aligned) {
        topF = elementRect.bottom;
        leftF = elementRect.left;
      } else {
        leftF = elementRect.right;
        topF = elementRect.top + (elementRect.height / 2);
      }

      // Work in rounded pixels for decisions to avoid boundary jitter
      const maxRight = window.innerWidth - SCREEN_PADDING;
      let left = Math.round(leftF);

      if (left + menuWidth > maxRight) {
        left = Math.round(elementRect.left - menuWidth);
      }

      if (left < SCREEN_PADDING) {
        left = SCREEN_PADDING;
      }

      const maxBottom = window.innerHeight - SCREEN_PADDING;
      let top = Math.round(topF);

      if (top + menuHeight > maxBottom) {
        if (bottom_aligned) {
          top = Math.round(elementRect.top - menuHeight);
        } else {
          top = Math.max(SCREEN_PADDING, Math.round(maxBottom - menuHeight));
        }
      }

      if (top < SCREEN_PADDING) {
        top = SCREEN_PADDING;
      }

      // Apply final integer pixels (consistent with checks above)
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
  position: fixed;
  left: 0;
  margin: 0;
  padding: 0;
  top: 0;
  z-index: 100;
  box-shadow: 2px 4px 6px rgba(0, 0, 0, 0.3);
}

</style>
