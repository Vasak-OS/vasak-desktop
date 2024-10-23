<script lang="ts" setup>
import { defineProps } from "vue";

defineProps({
  hours: {
    type: String,
    required: true,
  },
  minutes: {
    type: String,
    required: true,
  },
  seconds: {
    type: String,
    required: true,
  },
  amPm: {
    type: String,
    required: true,
  },
});
</script>

<template>
  <div class="clock-time">
    <span>{{ hours }}</span>
    <span>:</span>
    <span>{{ minutes }}</span>
    <span>:</span>
    <div class="clock-time-box">
      <span>{{ amPm }}</span>
      <span>{{ seconds }}</span>
    </div>
  </div>
</template>
