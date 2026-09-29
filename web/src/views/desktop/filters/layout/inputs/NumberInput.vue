<template>
  <div class="wrap">
    <input
      type="number"
      :value="localValue"
      :min="min"
      :max="max"
      :step="step"
      @input="onInput"
      @blur="onBlur"
    />

    <div class="overlay">
      <span class="ghost">{{ displayValue }}</span>
      <span v-if="suffix" class="suffix">{{ suffix }}</span>
    </div>
  </div>
</template>

<script>
export default {
  name: "NumberInput",
  emits: ["input"],

  props: {
    value: Number,
    min: {type: Number, default: -Infinity},
    max: {type: Number, default: Infinity},
    step: {type: Number, default: 1},
    suffix: String
  },

  data() {
    return {
      localValue: this.roundToStep(this.value)
    };
  },

  computed: {
    displayValue() {
      return this.localValue === null || this.localValue === undefined
        ? ""
        : String(this.localValue);
    }
  },

  watch: {
    value(v) {
      this.localValue = this.roundToStep(v);
    }
  },


  methods: {
    roundToStep(n) {
      if (!this.step || this.step === 0) return n;
      const decimals = (this.step.toString().split('.')[1] ?? '').length;
      return parseFloat(parseFloat(n).toFixed(decimals));
    },

    clamp(n) {
      return Math.min(this.max, Math.max(this.min, n));
    },

    snap(n) {
      if (!this.step || this.step === 0) return n;
      return parseFloat((Math.round(n / this.step) * this.step).toFixed(
        (this.step.toString().split('.')[1] ?? '').length
      ));
    },

    normalize(n) {
      return this.clamp(this.snap(n));
    },

    isValid(raw) {
      if (raw === "" || raw === "-") return false;
      const n = Number(raw);
      return !Number.isNaN(n);
    },

    onInput(e) {
      const raw = e.target.value;

      if (!this.isValid(raw)) {
        this.localValue = raw === "" ? null : raw;
        return;
      }

      const n = this.normalize(Number(raw));
      this.localValue = n;
      this.$emit("input", n);

      // Force the DOM element to reflect the clamped value
      this.$nextTick(() => {
        e.target.value = n;
      });
    },

    onBlur() {
      let n;

      if (this.localValue === null || this.localValue === undefined || this.localValue === "") {
        n = this.min > -Infinity ? this.min : 0;
      } else {
        n = Number(this.localValue);
        if (Number.isNaN(n)) {
          n = this.min > -Infinity ? this.min : 0;
        }
      }

      n = this.normalize(n);
      this.localValue = n;
      this.$emit("input", n);
    }
  }
};
</script>

<style scoped>
.wrap {
  position: relative;
  display: block;
  width: 100%;
}

.wrap input,
.overlay {
  font: inherit;
}

/* input fills parent correctly */
.wrap input {
  width: 100%;
  box-sizing: border-box;
  padding: 4px;
}

/* overlay does suffix layout only */
.overlay {
  position: absolute;
  inset: 0;

  display: flex;
  pointer-events: none;

  padding: 6px 6px;
  white-space: nowrap;
  box-sizing: border-box;
}

/* ghost defines width but stays invisible */
.ghost {
  color: transparent;
  white-space: pre;
}


/* suffix follows naturally */
.suffix {
  color: rgba(255, 255, 255, 0.5);
  margin-left: 2px;
  white-space: nowrap;
}
</style>
