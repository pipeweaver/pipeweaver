<script>
export default {
  name: "PopupBox",

  props: {},

  data() {
    return {
      is_active: false,
      suppress_click: true,
      identifier: null,
    };
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

      // We want to pop out from the right of whatever element was pressed, note that scrollTop has
      // to be considered, otherwise it'll pop low on scrolled pages..
      let left = positionElement.offsetLeft;
      let top = positionElement.offsetTop - scrollTop;

      if (bottom_aligned !== undefined && bottom_aligned) {
        top += positionElement.clientHeight;
      } else {
        // Now we need to position it to the bottom right of the element clicked..
        left += (positionElement.clientWidth);
        top += (positionElement.clientHeight / 2);
      }


      const container = this.$refs.container;
      this.identifier = identifier;

      let menuWidth = container.offsetWidth;
      let menuHeight = container.offsetHeight;

      let leftPosition = left + "px";
      let topPosition = top + "px";

      // Check if the Menu will break the window boundaries, and flip side if so.
      if (menuWidth + left >= window.innerWidth) {
        leftPosition = (left - menuWidth) + 'px';
      }

      let windowPosition = document.documentElement.scrollTop || document.body.scrollTop;
      if (menuHeight + top >= window.innerHeight + windowPosition) {
        topPosition = (top - menuHeight) + 'px';
      }

      // Set the Container
      container.style.left = leftPosition;
      container.style.top = topPosition;

      // Activate the Container on the next tick, to prevent click outside immediately closing it
      this.is_active = true;
    },

    hideDialog() {
      // There are odd cases when this can trigger twice, don't do it if we're not here anymore.
      if (this.is_active) {
        this.is_active = false;
        this.suppress_click = true;
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
