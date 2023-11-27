<script lang="ts">
import { defineComponent } from "vue";

export default defineComponent({
  name: "ClockDate",
  props: {
    day: {
      type: Number,
      required: true,
    },
    month: {
      type: Number,
      required: true,
    },
    year: {
      type: Number,
      required: true,
    },
  },
});
</script>

<template>
  <div class="clock-date text-center">
    <span class="clock-date-day">{{ day }}</span>
    <span>/</span>
    <span class="clock-date-month">{{ month }}</span>
    <span>/</span>
    <span class="clock-date-year">{{ year }}</span>
  </div>
</template>
