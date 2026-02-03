<template>
  <div v-show=is_visible class="modal-mask">
    <div class="modal-wrapper">
      <div ref="dialog" class="modal-container" role="dialog" aria-modal="true"
           :aria-labelledby="`${id}_label`"
           :aria-describedby="`${id}_body`" @keyup.esc.prevent="closeModalEsc">
        <div class="modal-header" tabindex="0">
          <div :id="`${id}_label`" role="heading" aria-level="2">
            <slot name="title" ref="title"></slot>
          </div>
          <button v-show=show_close ref="close" @click="closeModal()">
            <font-awesome-icon title="Close" icon="fa-solid fa-xmark"/>
          </button>
        </div>
        <div class="modal-body" :id="`${id}_body`">
          <slot></slot>
        </div>
        <div v-if="show_footer" class="modal-footer">
          <slot name="footer">
            <button ref="ok" class="modal-default-button" @click="closeModal()">
              {{ $t('message.modalButtons.ok') }}
            </button>
          </slot>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import * as focusTrap from "focus-trap";

export default {
  name: "ModalOverlay",
  emits: ["modal-close"],

  props: {
    id: {type: String, required: true},
    show_close: {type: Boolean, default: true},
    show_footer: {type: Boolean, default: true},
    prevent_esc: {type: Boolean, default: false},

    bodyPadding: {type: String, default: "20px"},
    width: {type: String, default: "500px"}
  },

  data() {
    return {
      is_visible: false,
      returnFocus: undefined,
      trap: undefined,
    }
  },

  methods: {
    openModal(focusRef, returnFocus) {
      this.returnFocus = returnFocus;
      this.is_visible = true;

      this.$nextTick(() => {
        if (focusRef === undefined) {
          if (this.$refs.ok !== undefined) {
            this.$refs.ok.focus();
          } else {
            this.$refs.close.focus();
          }
        } else {
          focusRef.focus();
        }

        // Create the focus trap, to prevent moving out..
        this.trap = focusTrap.createFocusTrap(this.$refs.dialog);
        this.trap.activate();
      })
    },

    closeModalEsc() {
      if (this.prevent_esc) {
        return;
      }
      this.closeModal()
    },

    closeModal() {
      // Deactivate the Trap (if active)..
      this.trap.deactivate();

      // Hide the UI..
      this.is_visible = false;

      // Return focus to the requested element.
      if (this.returnFocus !== undefined) {
        this.returnFocus.focus();
      }

      this.$emit('modal-close');
    },

    isOpen() {
      return this.is_visible;
    }
  }
}


</script>

<style scoped>
/* Turn the background of the screen grey */
.modal-mask {
  position: fixed;
  z-index: 9998;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: rgba(0, 0, 0, 0.2);
  display: table;
  transition: opacity 0.3s ease;
}

/* Positions the Modal in the Middle of the Screen */
.modal-wrapper {
  display: table-cell;
  vertical-align: middle;
}

/* The Actual Border / Setup of the Modal */
.modal-container {
  border: 1px solid #000;

  min-width: v-bind(width);
  max-width: min-content;
  margin: 0 auto;
  background-color: #fff;
  border-radius: 2px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.33);
  transition: all 0.3s ease;
}

/* Header Styling.. */
.modal-header {
  text-transform: uppercase;
  font-family: LeagueMono, sans-serif;
  background-color: #3b413f;
  text-align: center;
  color: #fff;
  overflow: hidden;
  outline: none;
}

.modal-header div {
  padding: 15px;
  float: left;
  display: block;
  clear: both;
}

.modal-header button {
  padding: 14px;
  float: right;
  cursor: pointer;
  background-color: transparent;
  color: #fff;
  border: 0;
}

.modal-body {
  background-color: #2d3230;
  color: #fff;
  padding: v-bind(bodyPadding);
}

.modal-footer {
  background-color: #2d3230;
  text-align: right;
  padding-right: 10px;
  padding-bottom: 10px;
}

.modal-footer button {
  background-color: #353937;
  color: #fff;
  padding: 8px 30px;
  border: none;
}

.modal-footer button:hover {
  background-color: #737775;
}
</style>
