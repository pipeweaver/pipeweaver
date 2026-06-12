<script>
import PopupBox from "@/views/desktop/inputs/PopupBox.vue";
import {get_device_by_id, getFullTargetList} from "@/app/util.js";
import {websocket} from "@/app/sockets.js";

export default {
  name: "DropMenu",
  emits: ['value-clicked', 'closed'],
  components: {PopupBox},

  props: {
    selected: {type: String, required: true},
    values: {type: Array, default: () => []},
  },

  data() {
    return {
      selectorWidth: 0,
    }
  },

  methods: {
    open_selector(e) {
      const anchor = (e && e.currentTarget) ? e.currentTarget : (e && e.target) ? e.target : undefined;
      const event = Object.assign({}, e, {target: anchor});
      if (this.$refs.popup) {
        // pass the anchor element (instead of the raw target)
        this.$refs.popup.showDialog(event, undefined, undefined, true);
      }
    },

    is_active(value) {
      return this.selected === value;
    },

    show(e) {
      this.$refs.popup.showDialog(e, this.id)
    },

    value_clicked(value) {
      this.$emit('value-clicked', value);
      this.$refs['popup'].close();
    },

    get_name_from_value(value) {
      return this.values.find(v => v.value === value).text;
    },

    onClosed(e) {
      this.$emit('closed', e);
    }
  },

  mounted() {
    const el = this.$refs.selector.querySelector('.inner');
    this.selectorWidth = el.clientWidth;
  },

  computed: {
    maxValueNameWidth() {
      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      ctx.font = '14px sans-serif';

      let maxWidth = 0;
      this.values.forEach(value => {
        const width = ctx.measureText(value.text).width + 5;
        maxWidth = Math.max(maxWidth, width);
      });

      if (maxWidth + 10 > 95) {
        return '95px';
      }

      return maxWidth + 'px';
    },

    popupWidth() {
      const contentWidth = parseFloat(this.maxValueNameWidth) + 29;
      return `${Math.max(this.selectorWidth, contentWidth)}px`;
    }
  }
}
</script>

<template>
  <PopupBox ref="popup" @closed="onClosed" :style="{ minWidth: popupWidth }">
    <div v-for="value in values">
      <div
        class="entry"
        :class="{ 'selected': is_active(value.value) }"
        :style="{ 'min-width': `calc(${maxValueNameWidth} + 3px)` }"
        @click="value_clicked(value.value)">

        <span class="title">{{ value.text }}</span>
      </div>
    </div>
  </PopupBox>

  <div :style="{ minWidth: `calc(${maxValueNameWidth} + 25px)`}" ref="selector" class="selector">
    <div class="inner" @click="open_selector($event)">
      <span>{{ get_name_from_value(selected) }}</span>
      <font-awesome-icon :icon="['fas', 'angle-down']" class="selector-icon"/>
    </div>
  </div>
</template>

<style scoped>
.selector {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
}

.selector .inner {
  display: flex;
  width: 100%;
  padding: 4px;
  align-items: center;
  justify-content: space-between;
  border: 1px solid #666;
  box-sizing: border-box;
}

.selector .inner:hover {
  background-color: #3b413f;
  cursor: pointer;
}

.title {
  white-space: nowrap;
}

.entry {
  white-space: nowrap;
  padding: 6px 20px 6px 6px;
}

.entry:hover {
  background-color: #49514e;
  cursor: pointer;
}

.entry:not(:last-child) {
  border-bottom: 1px solid #3b413f;
}

.selected {
  background-color: #214283;
}

</style>
